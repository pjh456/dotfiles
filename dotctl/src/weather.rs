use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{rofi, waybar::WaybarOutput};

const LAT: &str = "35.775";
const LON: &str = "120.030";
const PLACE: &str = "中国海洋大学西海岸校区";
const TTL: u64 = 1500;
const TIMEOUT: &str = "6";

const I_SUN: &str = "\u{f185}";
const I_PART: &str = "\u{eef0}";
const I_CLOUD: &str = "\u{f0c2}";
const I_FOG: &str = "\u{ef29}";
const I_RAIN: &str = "\u{ef1c}";
const I_SHOWER: &str = "\u{ef1d}";
const I_SNOW: &str = "\u{f2dc}";
const I_THUN: &str = "\u{ef2c}";
const I_DRIZ: &str = "\u{f043}";

const FORECAST_THEME: &str = r#"
    window { width: 780px; border-radius: 16px; border: 2px; border-color: #89b4fa55; background-color: #181825e6; }
    mainbox { padding: 14px; spacing: 10px; }
    listview { lines: 8; columns: 1; spacing: 4px; }
    element { padding: 8px 14px; border-radius: 10px; }
    element normal.normal { background-color: transparent; text-color: #cdd6f4; }
    element alternate.normal { background-color: transparent; text-color: #cdd6f4; }
    element selected.normal { background-color: #313244cc; text-color: #cdd6f4; border: 1px; border-color: #89b4fa66; }
    element-text { font: "JetBrainsMono Nerd Font 12"; }
"#;

#[derive(Debug, Deserialize)]
struct Weather {
    current: Current,
    #[serde(default)]
    daily: Option<Daily>,
}

#[derive(Debug, Deserialize)]
struct Current {
    temperature_2m: f64,
    weather_code: u16,
    apparent_temperature: f64,
    relative_humidity_2m: f64,
    wind_speed_10m: f64,
}

#[derive(Debug, Default, Deserialize)]
struct Daily {
    #[serde(default)]
    time: Vec<String>,
    #[serde(default)]
    weather_code: Vec<u16>,
    #[serde(default)]
    temperature_2m_max: Vec<f64>,
    #[serde(default)]
    temperature_2m_min: Vec<f64>,
    #[serde(default)]
    precipitation_probability_max: Vec<Option<f64>>,
    #[serde(default)]
    wind_speed_10m_max: Vec<Option<f64>>,
    #[serde(default)]
    sunrise: Vec<String>,
    #[serde(default)]
    sunset: Vec<String>,
}

pub fn status() -> Result<()> {
    let cache = cache_path()?;

    if let Some(value) = read_cache(&cache) {
        if cache_fresh(&value) {
            if emit_current(&value)? {
                return Ok(());
            }
        } else {
            spawn_refresh();
            if emit_current(&value)? {
                return Ok(());
            }
        }
    }

    if let Err(err) = refresh() {
        eprintln!("dotctl: weather refresh failed: {err:#}");
    }

    if let Some(value) = read_cache(&cache)
        && emit_current(&value)?
    {
        return Ok(());
    }

    println!(
        "{}",
        serde_json::to_string(&json!({
            "text": "",
            "tooltip": format!("{PLACE}: 无法获取数据"),
        }))?
    );
    Ok(())
}

pub fn refresh() -> Result<()> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={LAT}&longitude={LON}&timezone=Asia%2FShanghai&forecast_days=7&current=temperature_2m,weather_code,apparent_temperature,relative_humidity_2m,wind_speed_10m&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max,precipitation_sum,sunrise,sunset,wind_speed_10m_max"
    );
    let output = Command::new("curl")
        .args(["-sf", "--max-time", TIMEOUT, &url])
        .output()
        .context("run curl")?;
    if !output.status.success() {
        bail!("curl failed with {}", output.status);
    }
    if output.stdout.is_empty() {
        bail!("weather API returned an empty body");
    }

    let mut value: Value =
        serde_json::from_slice(&output.stdout).context("parse weather API response")?;
    value["ts"] = json!(now());

    let cache = cache_path()?;
    if let Some(parent) = cache.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let tmp = cache.with_extension(format!("json.{}.tmp", std::process::id()));
    fs::write(&tmp, serde_json::to_vec(&value)?)
        .with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, &cache).with_context(|| format!("replace {}", cache.display()))?;
    Ok(())
}

pub fn forecast() -> Result<()> {
    if rofi::close_if_open() {
        return Ok(());
    }

    let cache = cache_path()?;
    let mut value = read_cache(&cache);

    if value
        .as_ref()
        .and_then(|value| value.pointer("/daily/time"))
        .is_none()
    {
        let _ = refresh();
        value = read_cache(&cache);
    }

    let Some(value) = value else {
        return no_data();
    };
    let weather: Weather = match serde_json::from_value(value) {
        Ok(weather) => weather,
        Err(_) => return no_data(),
    };
    let Some(daily) = weather.daily else {
        return no_data();
    };
    if daily.time.is_empty() {
        return no_data();
    }

    let rows = render_rows(&weather.current, &daily);
    let args = vec![
        "-dmenu".to_owned(),
        "-markup-rows".to_owned(),
        "-p".to_owned(),
        " 天气".to_owned(),
        "-theme-str".to_owned(),
        FORECAST_THEME.to_owned(),
    ];
    rofi::run_with_input(&args, rows.as_bytes())?;
    Ok(())
}

fn no_data() -> Result<()> {
    let args = vec![
        "-dmenu".to_owned(),
        "-p".to_owned(),
        " 天气".to_owned(),
        "-mesg".to_owned(),
        format!("{PLACE}: 无法获取数据"),
    ];
    rofi::run_with_input(&args, b"")?;
    Ok(())
}

fn emit_current(value: &Value) -> Result<bool> {
    let weather: Weather = match serde_json::from_value(value.clone()) {
        Ok(weather) => weather,
        Err(_) => return Ok(false),
    };
    let current = &weather.current;
    let (icon, desc) = wmo(current.weather_code);
    let temp = fmt_num(current.temperature_2m);

    let output = WaybarOutput::new(
        format!(" {icon}  {temp}°C"),
        format!("wmo{}", current.weather_code),
    )
    .with_tooltip(format!(
        "{PLACE}\n{desc}  {temp}°C (体感 {}°C)\n湿度 {}%",
        fmt_num(current.apparent_temperature),
        fmt_num(current.relative_humidity_2m)
    ));
    output.print();
    Ok(true)
}

fn render_rows(current: &Current, daily: &Daily) -> String {
    let (cur_icon, cur_desc) = wmo(current.weather_code);
    let today = local_today();
    let mut lines = Vec::with_capacity(daily.time.len() + 1);

    lines.push(format!(
        "<span foreground=\"#cba6f7\" weight=\"bold\">{PLACE}</span>  {cur_icon} {cur_desc} <span foreground=\"#f9e2af\">{}°C</span>  <span foreground=\"#9399b2\">体感 {}° 湿度 {}% 风 {}km/h</span>",
        fmt_num(current.temperature_2m),
        fmt_num(current.apparent_temperature),
        fmt_num(current.relative_humidity_2m),
        fmt_num(current.wind_speed_10m),
    ));

    for (index, date) in daily.time.iter().enumerate() {
        let Some(code) = daily.weather_code.get(index).copied() else {
            continue;
        };
        let Some(tmax) = daily.temperature_2m_max.get(index).copied() else {
            continue;
        };
        let Some(tmin) = daily.temperature_2m_min.get(index).copied() else {
            continue;
        };

        let (icon, desc) = wmo_forecast(code);
        let (label, color) = if date == &today {
            ("今天".to_owned(), "#89b4fa")
        } else if index == 1 {
            ("明天".to_owned(), "#a6e3a1")
        } else {
            (weekday_label(date), "#cdd6f4")
        };

        let md = month_day(date);
        let pop = daily
            .precipitation_probability_max
            .get(index)
            .copied()
            .flatten();
        let wind = daily
            .wind_speed_10m_max
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(0.0);
        let sunrise = time_part(daily.sunrise.get(index).map(String::as_str));
        let sunset = time_part(daily.sunset.get(index).map(String::as_str));

        lines.push(format!(
            "<span foreground=\"{color}\" weight=\"bold\">{label} {md}</span>  {icon}  <span foreground=\"#f9e2af\">{tmax_w}°</span> / <span foreground=\"#89b4fa\">{tmin_w}°</span>  <span foreground=\"#94e2d5\">{pop_w}%</span>  <span foreground=\"#9399b2\">\u{f059d} {wind_w} km/h  {sunrise}-{sunset}</span>  <span foreground=\"#a6adc8\">{desc}</span>",
            tmax_w = width_round(tmax),
            tmin_w = width_round(tmin),
            pop_w = match pop {
                Some(value) => width_round(value),
                None => "  -".to_owned(),
            },
            wind_w = width_round(wind),
        ));
    }

    lines.join("\n")
}

fn width_round(value: f64) -> String {
    format!("{:>3}", value.round() as i64)
}

fn fmt_num(value: f64) -> String {
    format!("{value}")
}

fn wmo(code: u16) -> (&'static str, &'static str) {
    match code {
        0 => (I_SUN, "晴"),
        1 => (I_PART, "晴间多云"),
        2 => (I_PART, "多云"),
        3 => (I_CLOUD, "阴"),
        45 | 48 => (I_FOG, "雾"),
        51 | 53 | 55 | 56 | 57 => (I_DRIZ, "毛毛雨"),
        61 | 63 | 65 | 66 | 67 => (I_RAIN, "雨"),
        71 | 73 | 75 | 77 => (I_SNOW, "雪"),
        80..=82 => (I_SHOWER, "阵雨"),
        85 | 86 => (I_SNOW, "阵雪"),
        95 | 96 | 99 => (I_THUN, "雷阵雨"),
        _ => (I_CLOUD, "未知"),
    }
}

fn wmo_forecast(code: u16) -> (&'static str, &'static str) {
    match code {
        51 | 53 | 55 => (I_DRIZ, "毛毛雨"),
        56 | 57 => (I_DRIZ, "冻雨"),
        66 | 67 => (I_RAIN, "冻雨"),
        other => wmo(other),
    }
}

fn cache_path() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(dir).join("waybar-weather.json"));
    }
    let home = env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".cache")
        .join("waybar-weather.json"))
}

fn read_cache(path: &std::path::Path) -> Option<Value> {
    let data = fs::read(path).ok()?;
    serde_json::from_slice(&data).ok()
}

fn cache_fresh(value: &Value) -> bool {
    let Some(ts) = value.get("ts").and_then(Value::as_u64) else {
        return false;
    };
    now().saturating_sub(ts) < TTL
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn spawn_refresh() {
    let Ok(exe) = env::current_exe() else {
        return;
    };
    let _ = Command::new(exe)
        .args(["weather", "--refresh"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn local_today() -> String {
    // SAFETY: localtime_r writes into the supplied `tm` and does not retain the
    // pointer. `time` and the pointer are valid for the duration of the call.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return String::new();
        }
        format!(
            "{:04}-{:02}-{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday
        )
    }
}

fn month_day(date: &str) -> String {
    let mut parts = date.split('-');
    let (Some(_year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return date.to_owned();
    };
    format!("{month}/{day}")
}

fn weekday_label(date: &str) -> String {
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return "?　".to_owned();
    };
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<i64>(),
        month.parse::<i64>(),
        day.parse::<i64>(),
    ) else {
        return "?　".to_owned();
    };
    let weekday = (days_from_civil(year, month, day) + 3).rem_euclid(7) + 1;
    match weekday {
        1 => "一　",
        2 => "二　",
        3 => "三　",
        4 => "四　",
        5 => "五　",
        6 => "六　",
        7 => "日　",
        _ => "?　",
    }
    .to_owned()
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn time_part(value: Option<&str>) -> String {
    value
        .and_then(|value| value.split_once('T').map(|(_, time)| time))
        .unwrap_or("-")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekday_matches_iso() {
        assert_eq!(weekday_label("2026-11-28"), "六　");
        assert_eq!(weekday_label("2026-11-29"), "日　");
        assert_eq!(weekday_label("2026-11-30"), "一　");
    }

    #[test]
    fn month_day_keeps_leading_zero() {
        assert_eq!(month_day("2026-01-02"), "01/02");
    }

    #[test]
    fn forecast_widths_are_three_columns() {
        assert_eq!(width_round(5.4), "  5");
        assert_eq!(width_round(15.6), " 16");
        assert_eq!(width_round(-2.2), " -2");
    }
    #[test]
    fn forecast_renders_header_and_seven_days() {
        let current = Current {
            temperature_2m: 5.4,
            weather_code: 0,
            apparent_temperature: 0.4,
            relative_humidity_2m: 54.0,
            wind_speed_10m: 17.0,
        };
        let mut dates = vec![local_today()];
        dates.extend((1..7).map(|day| format!("2026-12-{day:02}")));
        let daily = Daily {
            time: dates,
            weather_code: vec![0, 2, 61, 71, 80, 95, 3],
            temperature_2m_max: vec![8.0, 9.4, 7.6, 1.0, 6.2, 5.0, 4.4],
            temperature_2m_min: vec![-1.0, 0.2, 2.6, -3.4, 1.0, 0.0, -0.6],
            precipitation_probability_max: vec![
                Some(0.0),
                None,
                Some(80.0),
                Some(60.0),
                Some(40.0),
                Some(90.0),
                Some(10.0),
            ],
            wind_speed_10m_max: vec![
                Some(12.0),
                Some(18.6),
                Some(22.0),
                Some(15.0),
                Some(10.0),
                Some(30.0),
                Some(9.0),
            ],
            sunrise: vec!["2026-11-28T06:50".to_owned(); 7],
            sunset: vec!["2026-11-28T16:45".to_owned(); 7],
        };

        let rows = render_rows(&current, &daily);
        assert_eq!(rows.lines().count(), 8);
        assert!(rows.contains("中国海洋大学西海岸校区"));
        assert!(rows.contains(&format!("今天 {}", month_day(&local_today()))));
        assert!(rows.contains("明天 "));
        assert!(rows.contains("  -%"));
        assert!(rows.contains("06:50-16:45"));
    }
}
