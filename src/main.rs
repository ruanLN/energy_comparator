use anyhow::{Ok, Result};
use chrono::{Datelike, NaiveDateTime, NaiveTime, Weekday};
use serde::Deserialize;
use std::{collections::HashSet, fs::File, io::BufReader, ops::Add, fmt::Debug};

// Defines the signature for the functions to define the price for a plan
trait PricePlanStrategy : Debug {
    fn price_for_singe_period(&self, datapoint: &SmartMeterData) -> EnergyBillEntry;
    fn standing_charge_per_day(&self) -> EnergyBillEntry;
    fn standing_charge_per_number_of_days(&self, days: u32) -> EnergyBillEntry {
        match self.standing_charge_per_day() {
            EnergyBillEntry::Credit(_) => panic!("we shouldnever get credit per dau"),
            EnergyBillEntry::Debit(day_value) => EnergyBillEntry::Debit(day_value * days as f32),
        }
    }

    fn compute_total_bill_for_period(&self, datapoints: &Vec<SmartMeterData>) -> EnergyBillEntry {
        datapoints
            .iter()
            .fold(EnergyBillEntry::Debit(0.0), |acc, d| {
                let price_for_singe_period = self.price_for_singe_period(d);
                let energy_bill_entry = acc + price_for_singe_period;
                energy_bill_entry
            })
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
struct SSEAirtricity20;
impl PricePlanStrategy for SSEAirtricity20 {
    fn price_for_singe_period(&self, datapoint: &SmartMeterData) -> EnergyBillEntry {
        const PEAK_ENERGY_START_TIME: NaiveTime = match NaiveTime::from_hms_opt(17, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };
        const PEAK_ENERGY_END_TIME: NaiveTime = match NaiveTime::from_hms_opt(19, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };

        const NIGHT_ENERGY_START_TIME: NaiveTime = match NaiveTime::from_hms_opt(23, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };
        const NIGHT_ENERGY_END_TIME: NaiveTime = match NaiveTime::from_hms_opt(8, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };

        match datapoint.read_type {
            SmartMeterDataType::ActiveImport => {
                if datapoint.read_data_and_end_time.time() > PEAK_ENERGY_START_TIME
                    && datapoint.read_data_and_end_time.time() <= PEAK_ENERGY_END_TIME
                {
                    EnergyBillEntry::Debit(0.4882 * (1.0 - 0.20) * datapoint.read_value)
                } else if datapoint.read_data_and_end_time.time() > NIGHT_ENERGY_START_TIME
                    && datapoint.read_data_and_end_time.time() <= NIGHT_ENERGY_END_TIME
                {
                    EnergyBillEntry::Debit(0.2506 * (1.0 - 0.20) * datapoint.read_value)
                } else {
                    EnergyBillEntry::Debit(0.3865 * (1.0 - 0.20) * datapoint.read_value)
                }
            }
            SmartMeterDataType::ActiveExport => {
                EnergyBillEntry::Credit(0.24 * datapoint.read_value)
            }
        }
    }

    fn standing_charge_per_day(&self) -> EnergyBillEntry {
        EnergyBillEntry::Debit(0.6602)
    }
}

#[derive(Debug)]
struct BordGaisEnergy25WeekendFree;
impl PricePlanStrategy for BordGaisEnergy25WeekendFree {
    /**
        Urban Day units (8am to 11pm)    43.04 35.30 cent per kWh
        Urban Peak units (5pm to 7pm)    52.58 43.12 cent per kWh
        Urban Night units (11pm to 8am)  31.63 25.94 cent per kWh
        Annual Standing Charge           €237.56
    */
    fn price_for_singe_period(&self, datapoint: &SmartMeterData) -> EnergyBillEntry {
        const FREE_ENERGY_START_TIME: NaiveTime = match NaiveTime::from_hms_opt(9, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };
        const FREE_ENERGY_END_TIME: NaiveTime = match NaiveTime::from_hms_opt(18, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };

        const PEAK_ENERGY_START_TIME: NaiveTime = match NaiveTime::from_hms_opt(17, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };
        const PEAK_ENERGY_END_TIME: NaiveTime = match NaiveTime::from_hms_opt(19, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };

        const NIGHT_ENERGY_START_TIME: NaiveTime = match NaiveTime::from_hms_opt(23, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };
        const NIGHT_ENERGY_END_TIME: NaiveTime = match NaiveTime::from_hms_opt(8, 0, 0) {
            Some(t) => t,
            None => panic!("Must be a valid time"),
        };

        const WEEKDAYS: [Weekday; 5] = [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
        ];
        match datapoint.read_type {
            SmartMeterDataType::ActiveImport => {
                // treat the sunday special case
                // free from 9am to 5pm
                // no peak time on weekends
                if datapoint.read_data_and_end_time.weekday() == Weekday::Sun
                    && datapoint.read_data_and_end_time.time() > FREE_ENERGY_START_TIME
                    && datapoint.read_data_and_end_time.time() <= FREE_ENERGY_END_TIME
                {
                    EnergyBillEntry::Debit(0.0)
                } else if WEEKDAYS.contains(&datapoint.read_data_and_end_time.weekday())
                    && datapoint.read_data_and_end_time.time() > PEAK_ENERGY_START_TIME
                    && datapoint.read_data_and_end_time.time() <= PEAK_ENERGY_END_TIME
                {
                    EnergyBillEntry::Debit(0.5258 * (1.0 - 0.25) * datapoint.read_value)
                } else if datapoint.read_data_and_end_time.time() > NIGHT_ENERGY_START_TIME
                    && datapoint.read_data_and_end_time.time() <= NIGHT_ENERGY_END_TIME
                {
                    EnergyBillEntry::Debit(0.3163 * (1.0 - 0.25) * datapoint.read_value)
                } else {
                    EnergyBillEntry::Debit(0.4304 * (1.0 - 0.25) * datapoint.read_value)
                }
            }
            SmartMeterDataType::ActiveExport => {
                EnergyBillEntry::Credit(0.185 * datapoint.read_value)
            }
        }
    }

    fn standing_charge_per_day(&self) -> EnergyBillEntry {
        EnergyBillEntry::Debit(237.56 / 365f32)
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
    const FILENAME: &str = "data/HDF_10308375697_09-01-2024.csv";
    let f = File::open(FILENAME)?;
    let reader = BufReader::new(f);
    // Build the CSV reader and iterate over each record.
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);
    let data: Vec<SmartMeterData> = rdr.deserialize().flat_map(|x| x).collect();

    let plans: Vec<Box<dyn PricePlanStrategy>> = vec![
        Box::new(ElectricIrelandHomeElectric14),
        Box::new(SSEAirtricity20),
        Box::new(BordGaisEnergy25WeekendFree),
    ];
    for plan in plans {
        let total = plan.compute_total_bill_for_period(&data)
            + plan.standing_charge_per_number_of_days(300);
        println!("{plan:?}: {total:?}");
    }

    Ok(())
}
