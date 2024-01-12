use anyhow::{Ok, Result};
use chrono::NaiveDateTime;
use serde::Deserialize;
use std::{fs::File, io::BufReader};

fn smart_meter_datetime_desserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct CustomVisitor;

    impl<'de> serde::de::Visitor<'de> for CustomVisitor {
        type Value = NaiveDateTime;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a datetime in the format %d-%m-%Y %H:%M")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            NaiveDateTime::parse_from_str(value, "%d-%m-%Y %H:%M").map_err(E::custom)
        }
    }

    deserializer.deserialize_str(CustomVisitor)
}

#[derive(Debug, PartialEq, Deserialize)]
enum SmartMeterDataType {
    #[serde(rename = "Active Import Interval (kW)")]
    ActiveImport,
    #[serde(rename = "Active Export Interval (kW)")]
    ActiveExport,
}

#[derive(Debug, Deserialize)]
struct SmartMeterData {
    //format:
    // MPRN,Meter Serial Number,Read Value,Read Type,Read Date and End Time
    // 10308375697,34996871,0,Active Export Interval (kW),08-01-2024 03:30
    #[serde(rename = "MPRN")]
    mprn: String,
    #[serde(rename = "Meter Serial Number")]
    meter_serial_number: String,
    #[serde(rename = "Read Value")]
    read_value: f32,
    #[serde(rename = "Read Type")]
    read_type: SmartMeterDataType,
    #[serde(
        rename = "Read Date and End Time",
        deserialize_with = "smart_meter_datetime_desserialize"
    )]
    read_data_and_end_time: NaiveDateTime,
}


fn main() -> Result<()> {
    const FILENAME: &str = "data/HDF_reduced.csv";
    let f = File::open(FILENAME)?;
    let mut reader = BufReader::new(f);
    // Build the CSV reader and iterate over each record.
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);
    let data: Vec<SmartMeterData> = rdr.deserialize().flat_map(|x| x).collect();
    for item in data {
        println!("{item:?}");
    }
    Ok(())
}
