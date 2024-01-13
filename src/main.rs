use anyhow::{Ok, Result};
use chrono::NaiveDateTime;
use serde::Deserialize;
use std::{fs::File, io::BufReader, ops::Add};

// Defines the signature for the function to define the price for a single datapoint for a plan
trait PricePlanStrategy {
    fn price_for_singe_period(&self, datapoint: &SmartMeterData) -> EnergyBillEntry;
    fn standing_charge_per_day(&self) -> EnergyBillEntry;
}

fn compute_total_bill_for_perio<T>(
    datapoints: &Vec<SmartMeterData>,
    price_plan: &T,
) -> EnergyBillEntry
where
    T: PricePlanStrategy + Sized,
{
    datapoints
        .iter()
        .fold(EnergyBillEntry::Debit(0.0), |acc, d| {
            let price_for_singe_period = price_plan.price_for_singe_period(d);
            let energy_bill_entry = acc + price_for_singe_period;
            println!("{acc:?} + {price_for_singe_period:?} = {energy_bill_entry:?}");
            energy_bill_entry
        })
}

struct ElectricIrelandHomeElectric14;
impl PricePlanStrategy for ElectricIrelandHomeElectric14 {
    fn price_for_singe_period(&self, datapoint: &SmartMeterData) -> EnergyBillEntry {
        match datapoint.read_type {
            SmartMeterDataType::ActiveImport => {
                EnergyBillEntry::Debit(0.3895 * (1.0 - 0.14) * datapoint.read_value)
            }
            SmartMeterDataType::ActiveExport => {
                EnergyBillEntry::Credit(0.21 * datapoint.read_value)
            }
        }
    }

    fn standing_charge_per_day(&self) -> EnergyBillEntry {
        EnergyBillEntry::Debit(272.61 / 365f32)
    }
}

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

#[derive(Debug, Clone, Copy)]
enum EnergyBillEntry {
    Credit(f32),
    Debit(f32),
}

impl Add for EnergyBillEntry {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (EnergyBillEntry::Debit(self_value), EnergyBillEntry::Debit(rhs_value)) => {
                EnergyBillEntry::Debit(self_value + rhs_value)
            }
            (EnergyBillEntry::Credit(self_value), EnergyBillEntry::Credit(rhs_value)) => {
                EnergyBillEntry::Credit(self_value + rhs_value)
            }
            (EnergyBillEntry::Credit(self_value), EnergyBillEntry::Debit(rhs_value)) => {
                if self_value > rhs_value {
                    EnergyBillEntry::Credit(self_value - rhs_value)
                } else {
                    EnergyBillEntry::Debit(rhs_value - self_value)
                }
            }
            (EnergyBillEntry::Debit(self_value), EnergyBillEntry::Credit(rhs_value)) => {
                if rhs_value > self_value {
                    EnergyBillEntry::Credit(rhs_value - self_value)
                } else {
                    EnergyBillEntry::Debit(self_value - rhs_value)
                }
            }
        }
    }
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
    let reader = BufReader::new(f);
    // Build the CSV reader and iterate over each record.
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);
    let data: Vec<SmartMeterData> = rdr.deserialize().flat_map(|x| x).collect();

    let total = compute_total_bill_for_perio(&data, &ElectricIrelandHomeElectric14);
    println!("{total:?}");
    Ok(())
}
