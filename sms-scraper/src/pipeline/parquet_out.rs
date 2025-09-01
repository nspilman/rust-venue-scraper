use crate::pipeline::processing::pipeline_steps::ParsedRecord;
use anyhow::Result;
use chrono::{NaiveDate, Utc};
use parquet::basic::{Compression, LogicalType, Type as PhysicalType};
use parquet::column::writer::ColumnWriter;
use parquet::data_type::ByteArray;
use parquet::file::properties::WriterProperties;
use parquet::file::writer::{FileWriter, SerializedFileWriter};
use parquet::schema::types::{LogicalType as LT, SchemaDescriptor, Type, TypePtr};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

// Build a simple schema: source_id: utf8, day: utf8, record_json: utf8
fn build_schema() -> TypePtr {
    let fields = vec![
        Type::primitive_type_builder("source_id", PhysicalType::BYTE_ARRAY)
            .with_logical_type(Some(LT::String))
            .build()
            .unwrap(),
        Type::primitive_type_builder("day", PhysicalType::BYTE_ARRAY)
            .with_logical_type(Some(LT::String))
            .build()
            .unwrap(),
        Type::primitive_type_builder("record_json", PhysicalType::BYTE_ARRAY)
            .with_logical_type(Some(LT::String))
            .build()
            .unwrap(),
    ];
    Type::group_type_builder("schema")
        .with_fields(fields)
        .build()
        .unwrap()
        .into()
}

fn infer_day(rec: &ParsedRecord) -> String {
    // Try common fields for dates in record JSON; fallback to today's UTC date
    let v = &rec.record;
    let try_fields = [
        ["start", "date"].as_slice(),
        ["startDate"].as_slice(),
        ["date"].as_slice(),
        ["event_day"].as_slice(),
    ];
    for path in try_fields {
        let mut cur = v;
        let mut ok = true;
        for p in path {
            if let Some(n) = cur.get(*p) { cur = n; } else { ok = false; break; }
        }
        if ok {
            if let Some(s) = cur.as_str() {
                // Accept YYYY-MM-DD prefix if present
                if s.len() >= 10 { return s[..10].to_string(); }
                // Try parse RFC3339 and take date
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                    return dt.date_naive().to_string();
                }
            }
            if let Some(n) = cur.as_i64() {
                // treat as epoch seconds
                let dt = chrono::NaiveDateTime::from_timestamp_opt(n, 0)
                    .unwrap_or_else(|| chrono::NaiveDateTime::from_timestamp_opt(0,0).unwrap());
                return dt.date().to_string();
            }
        }
    }
    Utc::now().date_naive().to_string()
}

pub fn write_partitioned_parquet(records: &[ParsedRecord], out_root: &Path) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    if records.is_empty() { return Ok(written); }

    // Group by (source_id, day)
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<(String, String), Vec<&ParsedRecord>> = BTreeMap::new();
    for r in records {
        let day = infer_day(r);
        groups.entry((r.source_id.clone(), day)).or_default().push(r);
    }

    let schema = build_schema();
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD)
        .build();

    for ((source, day), recs) in groups {
        // Partition path: out_root/source=.../date=...
        let part_dir = out_root
            .join(format!("source={}", source))
            .join(format!("date={}", day));
        fs::create_dir_all(&part_dir)?;
        let ts = Utc::now().format("%Y%m%d_%H%M%S");
        let file_path = part_dir.join(format!("part-{}.parquet", ts));
        let file = File::create(&file_path)?;
        let mut writer = SerializedFileWriter::new(file, schema.clone(), props.clone())?;
        {
            let mut rg = writer.next_row_group()?;
            // Prepare columns
            let src_vals: Vec<ByteArray> = recs.iter().map(|r| ByteArray::from(r.source_id.as_str())).collect();
            let day_vals: Vec<ByteArray> = recs.iter().map(|_| ByteArray::from(day.as_str())).collect();
            let rec_vals: Vec<ByteArray> = recs.iter().map(|r| ByteArray::from(serde_json::to_string(&r.record).unwrap_or_else(|_| "{}".into()))).collect();

            // Write each column in order
            let mut col_index = 0;
            while let Some(mut col_writer) = rg.next_column()? {
                match col_writer {
                    ColumnWriter::ByteArrayColumnWriter(ref mut cw) => {
                        match col_index {
                            0 => { cw.write_batch(&src_vals, None, None)?; }
                            1 => { cw.write_batch(&day_vals, None, None)?; }
                            2 => { cw.write_batch(&rec_vals, None, None)?; }
                            _ => unreachable!(),
                        }
                        cw.close()?;
                    }
                    _ => unreachable!("Unexpected column type - schema mismatch"),
                }
                col_index += 1;
                rg.close_column(col_writer)?;
            }
            writer.close_row_group(rg)?;
        }
        writer.close()?;
        written.push(file_path);
    }

    Ok(written)
}
