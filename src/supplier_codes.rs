// src/supplier_codes.rs
//! Codes printed by chemical suppliers on their bottles (Sigma-Aldrich / Merck).
//!
//! Pure parsing, no database. Four shapes are seen in practice:
//!
//! 1. GS1 Data Matrix, e.g.
//!    `(01)04022536028606(15)230831(422)276(10)A1335066(240)1020661000`
//!    AI 01 GTIN, 15/17 expiry, 422 country of origin, 10 batch, 240 article.
//!    On the wire variable-length fields are closed by a GS byte (0x1D); the
//!    camera library may also hand us the human-readable form with brackets.
//! 2. Sigma text code: `C0750-100G,BCCC8969`, `W508209-100G-K,MKCF3567`,
//!    `W228605-1KG-K;MKCH5601;102136798` (article-size[-grade], batch, Pcode).
//! 3. Bare Merck article: 10 digits, `102066` + pack code `1000`.
//! 4. Bare GTIN (EAN-13 / GTIN-14) with a valid check digit.
//!
//! Pack sizes of Merck articles are a code, not a quantity: `1000` can be 1 kg
//! or 1 L, `2511` is a special 2.5 L pack. The unit is therefore left unknown
//! for them and is taken from the article's stored mapping or asked once.

use chrono::{Datelike, NaiveDate};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;

pub const GS: char = '\u{1d}';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeFormat {
    Gs1,
    SigmaText,
    MerckArticle,
    Gtin,
}

/// Article numbering family. `merck` articles are 10-digit codes (article + pack),
/// `sigma` ones are `article-size`. The same digits can exist in both families,
/// so the family is part of every article key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Series {
    Merck,
    Sigma,
    Other,
}

impl Series {
    pub fn as_str(&self) -> &'static str {
        match self {
            Series::Merck => "merck",
            Series::Sigma => "sigma",
            Series::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pack {
    /// As printed: `100G`, `2.5KG`, `1000`
    pub raw: String,
    /// Numeric part when it is a quantity (Sigma). For a Merck pack code it is the
    /// code's number and says nothing about the unit.
    pub value: Option<f64>,
    /// Canonical unit (`mg`, `g`, `kg`, `mL`, `L`); None when the code does not say.
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SupplierCode {
    pub format: CodeFormat,
    pub series: Series,
    pub catalog: Option<String>,
    pub pack: Option<Pack>,
    /// Grade suffix after the pack size (`K` in `100G-K`, kosher grade)
    pub grade_suffix: Option<String>,
    pub batch: Option<String>,
    /// AI 17 (use by) when present, else AI 15 (best before). Day 00 means end of month.
    pub expiry: Option<NaiveDate>,
    /// GTIN-14 (AI 01, or a scanned EAN-13 padded with a zero)
    pub gtin: Option<String>,
    /// ISO 3166 numeric country code from AI 422
    pub origin: Option<String>,
    /// Sigma's internal product/pack code (`Pcode` on the label)
    pub pcode: Option<String>,
    /// Things the person should double-check
    pub warnings: Vec<String>,
}

impl SupplierCode {
    fn empty(format: CodeFormat, series: Series) -> Self {
        SupplierCode {
            format,
            series,
            catalog: None,
            pack: None,
            grade_suffix: None,
            batch: None,
            expiry: None,
            gtin: None,
            origin: None,
            pcode: None,
            warnings: Vec::new(),
        }
    }
}

lazy_static! {
    static ref SIZE_RE: Regex =
        Regex::new(r"^(\d+(?:\.\d+)?)(KG|MG|UL|ML|G|L)$").unwrap();
    /// `ARTICLE-SIZE[-GRADE]`, article letters/digits (`W228605`, `38796`, `F408`)
    static ref SIGMA_ARTICLE_RE: Regex = Regex::new(
        r"^([A-Z0-9]{2,20})-(\d+(?:\.\d+)?(?:KG|MG|UL|ML|G|L))((?:-[A-Z0-9]{1,4})*)$"
    ).unwrap();
    static ref BATCH_RE: Regex = Regex::new(r"^[A-Z0-9]{4,20}$").unwrap();
    /// What AI 240 may hold, used to find where it ends when the GS bytes were lost.
    static ref AI240_TAIL_RE: Regex = Regex::new(
        r"^(\d{10}|[A-Z0-9]{2,20}-\d+(?:\.\d+)?(?:KG|MG|UL|ML|G|L)(?:-[A-Z0-9]{1,4})*)(422\d{3})?$"
    ).unwrap();

    // Catalog numbers as people type them into Chelate (import files, older batches)
    /// `1.00014.2500`, `1.00014`, also with a mistyped pack (`1.047610.500`)
    static ref TYPED_MERCK_DOTTED_RE: Regex = Regex::new(r"^([18])\.(\d{5})(?:[.\d]{0,8})$").unwrap();
    /// `1000142500`: article + pack, as in AI 240
    static ref TYPED_MERCK_10_RE: Regex = Regex::new(r"^[18]\d{9}$").unwrap();
    /// `M0512-500G`, `244023`, `W292907`, `RDD016-2.5KG`, `103943-250G` (a Merck article
    /// written the way the Sigma web shop shows it), `W228605-1KG-K`
    static ref TYPED_ARTICLE_RE: Regex = Regex::new(
        r"^([A-Z]{0,3}\d{2,6})(?:-(\d+(?:[.,]\d+)?(?:KG|MG|UL|ML|G|L)))?(?:-[A-Z0-9]{1,4})*$"
    ).unwrap();
    /// Merck batch numbers: `K47917114`, `I1421783`, `Z0891532`, `AM1912388`, `HC31835254`
    static ref MERCK_BATCH_RE: Regex = Regex::new(r"^[A-Z]{1,2}\d{6,9}$").unwrap();
    /// Sigma-Aldrich batch numbers: `MKCS4698`, `BCCK1832`, `STBF8846V`
    static ref SIGMA_BATCH_RE: Regex = Regex::new(r"^[A-Z]{3,4}\d{3,5}[A-Z]?$").unwrap();
}

/// What a catalog number typed into Chelate says about the article, in the
/// same `series` + `catalog` terms a scanned code uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArticleKey {
    Known { series: Series, catalog: String },
    /// Six digits starting with 1 or 8 are a Merck article (`103943` = 1.03943) or a
    /// Sigma one (`185361`); the batch number did not tell which.
    Ambiguous { catalog: String },
    Unrecognised,
}

/// Reads a typed catalog number. The batch number decides between Merck and
/// Sigma when the digits fit both, because their batch formats differ.
pub fn article_key(cat_number: &str, batch_number: Option<&str>) -> ArticleKey {
    let a: String = cat_number.trim().to_uppercase().chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(c) = TYPED_MERCK_DOTTED_RE.captures(&a) {
        return ArticleKey::Known { series: Series::Merck, catalog: format!("{}{}", &c[1], &c[2]) };
    }
    if TYPED_MERCK_10_RE.is_match(&a) {
        return ArticleKey::Known { series: Series::Merck, catalog: a[..6].to_string() };
    }
    let Some(c) = TYPED_ARTICLE_RE.captures(&a) else {
        return ArticleKey::Unrecognised;
    };
    let art = c[1].to_string();
    let merck_like = art.len() == 6 && art.chars().all(|ch| ch.is_ascii_digit()) && matches!(&art[..1], "1" | "8");
    if !merck_like {
        return ArticleKey::Known { series: Series::Sigma, catalog: art };
    }
    let batch = batch_number.map(|b| b.trim().to_uppercase()).unwrap_or_default();
    if MERCK_BATCH_RE.is_match(&batch) {
        ArticleKey::Known { series: Series::Merck, catalog: art }
    } else if SIGMA_BATCH_RE.is_match(&batch) {
        ArticleKey::Known { series: Series::Sigma, catalog: art }
    } else {
        ArticleKey::Ambiguous { catalog: art }
    }
}

// ==================== ENTRY POINT ====================

/// Parses a scanned or typed supplier code. None if it is not one.
///
/// `raw` should still contain GS bytes; scanners' CR/LF, the symbology identifier
/// (`]d2`) and a literal `<GS>` are tolerated.
pub fn parse_supplier_code(raw: &str) -> Option<SupplierCode> {
    let s = normalize(raw);
    if s.is_empty() || s.len() > 200 {
        return None;
    }

    if s.contains('(') {
        return parse_gs1_hri(&s);
    }
    if let Some(c) = parse_sigma_text(&s) {
        return Some(c);
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        if s.len() == 10 {
            return Some(parse_merck_article(&s));
        }
        if (s.len() == 13 || s.len() == 14) && gtin_check_ok(&s) {
            let mut c = SupplierCode::empty(CodeFormat::Gtin, Series::Other);
            c.gtin = Some(format!("{:0>14}", s));
            return Some(c);
        }
    }
    parse_gs1_raw(&s)
}

fn normalize(raw: &str) -> String {
    let mut s = raw.replace("<GS>", &GS.to_string()).replace('\u{241d}', &GS.to_string());
    s.retain(|c| !(c.is_control() && c != GS));
    let s = s.trim();
    // Symbology identifiers: ]d2 (Data Matrix), ]d1, ]C1 (GS1-128), ]Q3 (QR)
    let s = match s.as_bytes() {
        [b']', _, _, ..] if s.is_char_boundary(3) => &s[3..],
        _ => s,
    };
    s.trim_matches(GS).trim().to_string()
}

// ==================== GS1 ====================

enum AiKind {
    Fixed(usize),
    Var,
}

/// Length class of an application identifier at the start of `s`: (AI length in
/// characters, kind). Only the AIs that appear on laboratory products are listed;
/// an unknown AI stops the parse instead of guessing.
fn ai_at(s: &str) -> Option<(usize, AiKind)> {
    let b = s.as_bytes();
    if b.len() < 2 || !b[..2].iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    match &s[..2] {
        "00" => return Some((2, AiKind::Fixed(18))),
        "01" | "02" => return Some((2, AiKind::Fixed(14))),
        "11" | "12" | "13" | "14" | "15" | "16" | "17" | "18" | "19" => {
            return Some((2, AiKind::Fixed(6)))
        }
        "20" => return Some((2, AiKind::Fixed(2))),
        "10" | "21" | "22" | "30" | "37" | "90" => return Some((2, AiKind::Var)),
        _ => {}
    }
    if b.len() >= 3 && b[2].is_ascii_digit() {
        match &s[..3] {
            "422" | "424" | "426" => return Some((3, AiKind::Fixed(3))),
            "240" | "241" | "243" | "250" | "251" | "253" | "254" | "420" | "421" | "423"
            | "425" | "400" | "401" | "403" => return Some((3, AiKind::Var)),
            "242" => return Some((3, AiKind::Var)),
            "410" | "411" | "412" | "413" | "414" | "415" | "416" | "417" => {
                return Some((3, AiKind::Fixed(13)))
            }
            _ => {}
        }
    }
    None
}

/// Ordered (AI, value) pairs
type Fields = Vec<(String, String)>;

fn parse_gs1_raw(s: &str) -> Option<SupplierCode> {
    let has_gs = s.contains(GS);
    let mut fields: Fields = Vec::new();
    let mut warnings = Vec::new();
    let mut i = 0;

    while i < s.len() {
        if s[i..].starts_with(GS) {
            i += GS.len_utf8();
            continue;
        }
        let rest = &s[i..];
        let (ai_len, kind) = match ai_at(rest) {
            Some(x) => x,
            None if fields.is_empty() => return None,
            None => {
                warnings.push(format!("Unrecognised data after the known fields: \"{}\"", clip(rest)));
                break;
            }
        };
        let ai = rest[..ai_len].to_string();
        let body = &rest[ai_len..];
        match kind {
            AiKind::Fixed(n) => {
                if body.len() < n || !body.is_char_boundary(n) {
                    return if fields.is_empty() { None } else { finish(fields, warnings) };
                }
                fields.push((ai, body[..n].to_string()));
                i += ai_len + n;
            }
            AiKind::Var => {
                let end = body.find(GS).unwrap_or(body.len());
                let mut value = &body[..end];
                let mut consumed = end;
                if !has_gs {
                    // The GS bytes are gone: find the end of the field by what may follow
                    if let Some(cut) = split_without_gs(&ai, value) {
                        warnings.push(
                            "The code has no separators, so field boundaries were inferred. \
                             Check the batch number against the label."
                                .to_string(),
                        );
                        value = &body[..cut];
                        consumed = cut;
                    }
                }
                if value.is_empty() {
                    return None;
                }
                fields.push((ai, value.to_string()));
                i += ai_len + consumed;
            }
        }
    }
    finish(fields, warnings)
}

/// Where a variable field ends in a string without GS bytes. Only needed for AI 10
/// (batch, followed by the article) and AI 240 (article, followed by the origin).
fn split_without_gs(ai: &str, value: &str) -> Option<usize> {
    match ai {
        "10" => (4..value.len().saturating_sub(3))
            .filter(|p| value.is_char_boundary(*p))
            .find(|p| value[*p..].starts_with("240") && AI240_TAIL_RE.is_match(&value[*p + 3..]))
            ,
        "240" => AI240_TAIL_RE
            .captures(value)
            .and_then(|c| c.get(2).map(|m| m.start())),
        _ => None,
    }
}

fn parse_gs1_hri(s: &str) -> Option<SupplierCode> {
    lazy_static! {
        static ref PAIR: Regex = Regex::new(r"\((\d{2,4})\)([^(]*)").unwrap();
    }
    if !s.starts_with('(') {
        return None;
    }
    let mut fields: Fields = Vec::new();
    let mut covered = 0;
    for cap in PAIR.captures_iter(s) {
        let whole = cap.get(0).unwrap();
        if whole.start() != covered {
            return None;
        }
        covered = whole.end();
        let value = cap[2].trim_matches(GS).to_string();
        if value.is_empty() {
            return None;
        }
        fields.push((cap[1].to_string(), value));
    }
    if fields.is_empty() || covered != s.len() {
        return None;
    }
    finish(fields, Vec::new())
}

fn finish(fields: Fields, warnings: Vec<String>) -> Option<SupplierCode> {
    let get = |ai: &str| fields.iter().find(|(a, _)| a == ai).map(|(_, v)| v.as_str());

    let mut code = SupplierCode::empty(CodeFormat::Gs1, Series::Other);
    code.warnings = warnings;
    code.batch = get("10").map(|b| b.to_uppercase());
    code.gtin = get("01").map(|g| g.to_string());
    code.origin = get("422").map(|o| o.to_string());

    for (ai, label) in [("17", "use by"), ("15", "best before")] {
        if let Some(v) = get(ai) {
            match parse_gs1_date(v) {
                Some(d) => {
                    code.expiry = Some(d);
                    break;
                }
                None => code.warnings.push(format!("Unreadable {} date \"{}\"", label, v)),
            }
        }
    }

    if let Some(article) = get("240") {
        apply_article(&mut code, article);
    }

    // A code with neither an article nor a batch tells us nothing useful
    if code.catalog.is_none() && code.batch.is_none() && code.gtin.is_none() {
        return None;
    }
    if code.catalog.is_none() {
        code.warnings
            .push("The code has no article number (AI 240); the article is not known.".to_string());
    }
    Some(code)
}

fn apply_article(code: &mut SupplierCode, article: &str) {
    let a = article.trim().to_uppercase();
    if let Some(c) = SIGMA_ARTICLE_RE.captures(&a) {
        code.series = Series::Sigma;
        code.catalog = Some(c[1].to_string());
        code.pack = pack_from_size(&c[2]);
        let suffix = c[3].trim_start_matches('-');
        if !suffix.is_empty() {
            code.grade_suffix = Some(suffix.to_string());
        }
    } else if a.len() == 10 && a.chars().all(|c| c.is_ascii_digit()) {
        code.series = Series::Merck;
        code.catalog = Some(a[..6].to_string());
        code.pack = Some(merck_pack(&a[6..]));
    } else {
        code.catalog = Some(a.clone());
        code.warnings
            .push(format!("Unrecognised article format \"{}\"", clip(&a)));
    }
}

fn parse_gs1_date(v: &str) -> Option<NaiveDate> {
    if v.len() != 6 || !v.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let yy: i32 = v[0..2].parse().ok()?;
    let mm: u32 = v[2..4].parse().ok()?;
    let dd: u32 = v[4..6].parse().ok()?;
    let year = 2000 + yy;
    if dd == 0 {
        // GS1: day 00 = last day of the month
        let (ny, nm) = if mm == 12 { (year + 1, 1) } else { (year, mm + 1) };
        let first_next = NaiveDate::from_ymd_opt(ny, nm, 1)?;
        if mm == 0 {
            return None;
        }
        return first_next.pred_opt().filter(|d| d.year() == year);
    }
    NaiveDate::from_ymd_opt(year, mm, dd)
}

// ==================== SIGMA TEXT ====================

fn parse_sigma_text(s: &str) -> Option<SupplierCode> {
    let upper = s.to_uppercase();
    let parts: Vec<&str> = upper.split(|c| c == ',' || c == ';').map(|p| p.trim()).collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let caps = SIGMA_ARTICLE_RE.captures(parts[0])?;

    let mut code = SupplierCode::empty(CodeFormat::SigmaText, Series::Sigma);
    code.catalog = Some(caps[1].to_string());
    code.pack = pack_from_size(&caps[2]);
    let suffix = caps[3].trim_start_matches('-');
    if !suffix.is_empty() {
        code.grade_suffix = Some(suffix.to_string());
    }
    if let Some(b) = parts.get(1).filter(|b| !b.is_empty()) {
        if !BATCH_RE.is_match(b) {
            return None;
        }
        code.batch = Some(b.to_string());
    }
    if let Some(p) = parts.get(2).filter(|p| !p.is_empty()) {
        if !p.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        code.pcode = Some(p.to_string());
    }
    Some(code)
}

fn pack_from_size(size: &str) -> Option<Pack> {
    let c = SIZE_RE.captures(size)?;
    let unit = match &c[2] {
        "MG" => "mg",
        "G" => "g",
        "KG" => "kg",
        "ML" => "mL",
        "L" => "L",
        "UL" => "µL",
        _ => return None,
    };
    Some(Pack {
        raw: size.to_string(),
        value: c[1].parse().ok(),
        unit: Some(unit.to_string()),
    })
}

// ==================== MERCK ====================

fn parse_merck_article(s: &str) -> SupplierCode {
    let mut c = SupplierCode::empty(CodeFormat::MerckArticle, Series::Merck);
    c.catalog = Some(s[..6].to_string());
    c.pack = Some(merck_pack(&s[6..]));
    c
}

fn merck_pack(code: &str) -> Pack {
    Pack {
        raw: code.to_string(),
        value: code.parse().ok(),
        unit: None,
    }
}

// ==================== GTIN ====================

/// GS1 mod-10 check digit over a 13- or 14-digit string
pub fn gtin_check_ok(s: &str) -> bool {
    if !(s.len() == 13 || s.len() == 14) || !s.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    let (body, check) = digits.split_at(digits.len() - 1);
    let sum: u32 = body
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| if i % 2 == 0 { d * 3 } else { *d })
        .sum();
    (10 - sum % 10) % 10 == check[0]
}

/// CAS registry number with a valid check digit (`14371-10-9`)
pub fn cas_check_ok(s: &str) -> bool {
    lazy_static! {
        static ref CAS: Regex = Regex::new(r"^(\d{2,7})-(\d{2})-(\d)$").unwrap();
    }
    let Some(c) = CAS.captures(s.trim()) else { return false };
    let body = format!("{}{}", &c[1], &c[2]);
    let sum: u32 = body
        .chars()
        .rev()
        .filter_map(|d| d.to_digit(10))
        .enumerate()
        .map(|(i, d)| d * (i as u32 + 1))
        .sum();
    sum % 10 == c[3].parse::<u32>().unwrap_or(99)
}

fn clip(s: &str) -> String {
    s.chars().take(40).collect()
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn gs(s: &str) -> String {
        s.replace("␝", &GS.to_string())
    }

    #[test]
    fn real_gs1_codes_with_separators() {
        let list: Vec<(String, &str, &str, Option<&str>, &str, Option<(i32, u32, u32)>)> = vec![
            (gs("010402253602860615230831422276␝10A1335066␝2401020661000"), "102066", "1000", None, "A1335066", Some((2023, 8, 31))),
            (gs("010402253602860615230831422276␝10A1334966␝2401020661000"), "102066", "1000", None, "A1334966", Some((2023, 8, 31))),
            // the batch is 9 characters here, not 8: the old heuristic cut the last digit
            (gs("0104065268083710422276␝10K55640426␝2401054262511"), "105426", "2511", None, "K55640426", None),
            (gs("010402253664634315260430422276␝10HC31835254␝2401167545000"), "116754", "5000", None, "HC31835254", Some((2026, 4, 30))),
            (gs("010406183555518510STBL0017␝240452882-500G␝422840"), "452882", "500G", Some("g"), "STBL0017", None),
            (gs("0104022536060279422276␝10AM1993463␝2401050631000"), "105063", "1000", None, "AM1993463", None),
            (gs("010402253605961715261031422250␝10MB2225233␝2401050331000"), "105033", "1000", None, "MB2225233", Some((2026, 10, 31))),
            (gs("010402253603680915270331422276␝10AM1838590␝2401027901000"), "102790", "1000", None, "AM1838590", Some((2027, 3, 31))),
            (gs("010402253600453215280331422040␝10Z0866617␝2401003172500"), "100317", "2500", None, "Z0866617", Some((2028, 3, 31))),
            (gs("010405483901261715260228422710␝10K55121923␝2401096232511"), "109623", "2511", None, "K55121923", Some((2026, 2, 28))),
            (gs("010406183825207410STBK7206␝240131903-500ML␝422156"), "131903", "500ML", Some("mL"), "STBK7206", None),
            (gs("010406183556312810BCCJ6736␝240448869-50G␝422528"), "448869", "50G", Some("g"), "BCCJ6736", None),
            (gs("0104022536384597422156␝10S7346911␝2408043110100"), "804311", "0100", None, "S7346911", None),
            (gs("0104022536486499422530␝10S8089557␝2408222571000"), "822257", "1000", None, "S8089557", None),
            (gs("0104022536529233422380␝10S6073637␝2408147371000"), "814737", "1000", None, "S6073637", None),
            (gs("010402253600628415251130422276␝10Z0839541␝2401004411000"), "100441", "1000", None, "Z0839541", Some((2025, 11, 30))),
        ];
        for (raw, catalog, pack, unit, batch, exp) in list {
            let c = parse_supplier_code(&raw).unwrap_or_else(|| panic!("not parsed: {:?}", raw));
            assert_eq!(c.format, CodeFormat::Gs1, "{raw:?}");
            assert_eq!(c.catalog.as_deref(), Some(catalog), "{raw:?}");
            let p = c.pack.as_ref().unwrap_or_else(|| panic!("no pack: {raw:?}"));
            assert_eq!(p.raw, pack, "{raw:?}");
            assert_eq!(p.unit.as_deref(), unit, "{raw:?}");
            assert_eq!(c.batch.as_deref(), Some(batch), "{raw:?}");
            assert_eq!(c.expiry, exp.map(|(y, m, d)| ymd(y, m, d)), "{raw:?}");
            assert!(c.warnings.is_empty(), "{raw:?}: {:?}", c.warnings);
        }
    }

    #[test]
    fn series_and_origin_and_gtin() {
        let c = parse_supplier_code(&gs("010402253602860615230831422276␝10A1335066␝2401020661000")).unwrap();
        assert_eq!(c.series, Series::Merck);
        assert_eq!(c.gtin.as_deref(), Some("04022536028606"));
        assert_eq!(c.origin.as_deref(), Some("276"));
        let s = parse_supplier_code(&gs("010406183555518510STBL0017␝240452882-500G␝422840")).unwrap();
        assert_eq!(s.series, Series::Sigma);
        assert_eq!(s.origin.as_deref(), Some("840"));
    }

    #[test]
    fn human_readable_form_from_the_camera_library() {
        let c = parse_supplier_code("(01)04022536028606(15)230831(422)276(10)A1335066(240)1020661000").unwrap();
        assert_eq!(c.batch.as_deref(), Some("A1335066"));
        assert_eq!(c.catalog.as_deref(), Some("102066"));
        assert_eq!(c.expiry, Some(ymd(2023, 8, 31)));
        let s = parse_supplier_code("(01)04061835555185(10)STBL0017(240)452882-500G(422)840").unwrap();
        assert_eq!(s.catalog.as_deref(), Some("452882"));
        assert_eq!(s.pack.unwrap().unit.as_deref(), Some("g"));
    }

    #[test]
    fn lost_separators_are_inferred_and_flagged() {
        // Some tools drop the GS bytes when copying
        let c = parse_supplier_code("010406526808371042227610K556404262401054262511").unwrap();
        assert_eq!(c.batch.as_deref(), Some("K55640426"));
        assert_eq!(c.catalog.as_deref(), Some("105426"));
        assert!(!c.warnings.is_empty());

        let c = parse_supplier_code("010406183555518510STBL0017240452882-500G422840").unwrap();
        assert_eq!(c.batch.as_deref(), Some("STBL0017"));
        assert_eq!(c.catalog.as_deref(), Some("452882"));
        assert_eq!(c.pack.unwrap().raw, "500G");
    }

    #[test]
    fn sigma_text_codes() {
        let c = parse_supplier_code("38796-1L,BCCB0529").unwrap();
        assert_eq!(c.format, CodeFormat::SigmaText);
        assert_eq!((c.catalog.as_deref(), c.batch.as_deref()), (Some("38796"), Some("BCCB0529")));
        let p = c.pack.unwrap();
        assert_eq!((p.value, p.unit.as_deref()), (Some(1.0), Some("L")));

        let c = parse_supplier_code("795445-2.5KG,MKCH4059").unwrap();
        assert_eq!(c.pack.unwrap().value, Some(2.5));

        // -K grade suffix and a Pcode as third field
        let c = parse_supplier_code("W508209-100G-K,MKCF3567").unwrap();
        assert_eq!(c.catalog.as_deref(), Some("W508209"));
        assert_eq!(c.grade_suffix.as_deref(), Some("K"));
        assert_eq!(c.batch.as_deref(), Some("MKCF3567"));
        assert_eq!(c.pcode, None);

        let c = parse_supplier_code("W228605-1KG-K;MKCH5601;102136798").unwrap();
        assert_eq!(c.catalog.as_deref(), Some("W228605"));
        assert_eq!(c.pack.unwrap().raw, "1KG");
        assert_eq!(c.batch.as_deref(), Some("MKCH5601"));
        assert_eq!(c.pcode.as_deref(), Some("102136798"));

        // numeric batch
        let c = parse_supplier_code("452343-50G,0000117295").unwrap();
        assert_eq!(c.batch.as_deref(), Some("0000117295"));

        // article and size only
        let c = parse_supplier_code("PHR1234-500MG").unwrap();
        assert_eq!(c.batch, None);
        assert_eq!(c.pack.unwrap().unit.as_deref(), Some("mg"));
    }

    #[test]
    fn every_sigma_text_code_in_the_lab_export() {
        for raw in [
            "38796-1L,BCCB0529", "769290-100G,BCCB2190", "W508209-100G-K,MKCF3567",
            "185507-100G,BCCC3126", "D170801-1G,MKCC5118", "124931-100ML,MKCH2083",
            "147141-25G,MKCG0208", "F408-500G,STBH3795", "M57006-100G,STBG8884",
            "A7506-100G,SLCB4772", "W228605-1KG-K;MKCH5601;102136798", "793523-100G,MKCC7426",
            "157740-1KG,STBH4253", "W290017-1KG-K;MKCG4422;102143801", "795445-2.5KG,MKCH4059",
        ] {
            let c = parse_supplier_code(raw).unwrap_or_else(|| panic!("not parsed: {raw}"));
            assert!(c.catalog.is_some() && c.batch.is_some() && c.pack.is_some(), "{raw}: {c:?}");
        }
    }

    #[test]
    fn bare_merck_article_and_gtin() {
        let c = parse_supplier_code("1003172500").unwrap();
        assert_eq!(c.format, CodeFormat::MerckArticle);
        assert_eq!(c.catalog.as_deref(), Some("100317"));
        let p = c.pack.unwrap();
        assert_eq!((p.raw.as_str(), p.unit), ("2500", None));

        // EAN-13 scanned off the bottle equals the GTIN inside the Data Matrix
        let e = parse_supplier_code("4022536059617").unwrap();
        assert_eq!(e.format, CodeFormat::Gtin);
        assert_eq!(e.gtin.as_deref(), Some("04022536059617"));
        let dm = parse_supplier_code(&gs("010402253605961715261031422250␝10MB2225233␝2401050331000")).unwrap();
        assert_eq!(dm.gtin, e.gtin);

        // a digit string with a wrong check digit is not a GTIN
        assert!(parse_supplier_code("4022536059618").is_none());
    }

    #[test]
    fn scanner_noise_is_tolerated() {
        let raw = format!("]d2010402253602860615230831422276{GS}10A1335066{GS}2401020661000\r\n");
        assert_eq!(parse_supplier_code(&raw).unwrap().batch.as_deref(), Some("A1335066"));
        let raw = "010402253602860615230831422276<GS>10A1335066<GS>2401020661000";
        assert_eq!(parse_supplier_code(raw).unwrap().catalog.as_deref(), Some("102066"));
    }

    #[test]
    fn not_supplier_codes() {
        for raw in [
            "", "   ", "CHL:C:0123456789ABCDEF0123456789ABCDEF", "CHL:L:P:0123456789ABCDEF0123456789ABCDEF",
            "hello", "24870DF5", "P24018C5B", "(99)", "12345",
        ] {
            assert!(parse_supplier_code(raw).is_none(), "{raw:?} must not parse");
        }
    }

    #[test]
    fn expiry_end_of_month_and_bad_dates() {
        assert_eq!(parse_gs1_date("260200"), Some(ymd(2026, 2, 28)));
        assert_eq!(parse_gs1_date("280200"), Some(ymd(2028, 2, 29)));
        assert_eq!(parse_gs1_date("261200"), Some(ymd(2026, 12, 31)));
        assert_eq!(parse_gs1_date("261331"), None);
        assert_eq!(parse_gs1_date("260000"), None);
        assert_eq!(parse_gs1_date("2602"), None);
    }

    #[test]
    fn use_by_wins_over_best_before() {
        let c = parse_supplier_code("(01)04022536028606(15)230831(17)250101(10)A1335066(240)1020661000").unwrap();
        assert_eq!(c.expiry, Some(ymd(2025, 1, 1)));
    }

    #[test]
    fn cas_check_digit() {
        assert!(cas_check_ok("14371-10-9")); // trans-cinnamaldehyde, from the label
        assert!(cas_check_ok("64-17-5")); // ethanol
        assert!(cas_check_ok("7732-18-5")); // water
        assert!(!cas_check_ok("14371-10-8"));
        assert!(!cas_check_ok("abc"));
        assert!(!cas_check_ok("143711-09"));
    }

    #[test]
    fn typed_catalog_numbers() {
        let merck = |c: &str| ArticleKey::Known { series: Series::Merck, catalog: c.to_string() };
        let sigma = |c: &str| ArticleKey::Known { series: Series::Sigma, catalog: c.to_string() };
        // Forms seen in the lab's import file
        for (cat, batch, want) in [
            ("1000142500", "K47917114", merck("100014")),
            ("8017562500", "S8411256", merck("801756")),
            ("1.05426.1000", "", merck("105426")),
            ("8.22290.1000", "S8213890", merck("822290")),
            ("1.047610.500", "B2141561", merck("104761")),
            ("103943-250G", "B2162743", merck("103943")),
            ("105432-2511L", "Z0891532", merck("105432")),
            ("116754", "HC31835254", merck("116754")),
            ("101188-1KG", "AM1912388", merck("101188")),
            ("106498-5KG", "MB2226298", merck("106498")),
            ("812705", "70820", ArticleKey::Ambiguous { catalog: "812705".to_string() }),
            ("185361-500G", "MKCS4698", sigma("185361")),
            ("190764", "BCCK1832", sigma("190764")),
            ("M0512-500G", "0000431902", sigma("M0512")),
            ("209201-1KG", "1004123AZ", sigma("209201")),
            ("185361-500G", "1004123AZ", ArticleKey::Ambiguous { catalog: "185361".to_string() }),
            ("34850", "STBF8846V", sigma("34850")),
            ("W292907", "SHBP9989", sigma("W292907")),
            ("RDD016-2.5KG", "SLBZ3807", sigma("RDD016")),
            ("C805172-500g", "", sigma("C805172")),
            ("W228605-1KG-K", "MKCH5601", sigma("W228605")),
            ("06300-1KG", "", sigma("06300")),
            ("21390.293", "17I204006", ArticleKey::Unrecognised),
            ("ED2SS", "BCCH8763", ArticleKey::Unrecognised),
            ("2221228", "STBJ2608", ArticleKey::Unrecognised),
            ("", "X", ArticleKey::Unrecognised),
        ] {
            assert_eq!(article_key(cat, Some(batch)), want, "{cat} / {batch}");
        }
        // A scanned Merck code and a typed one give the same key
        let scanned = parse_supplier_code("1039430250").unwrap();
        assert_eq!(
            article_key("103943-250G", Some("B2162743")),
            ArticleKey::Known { series: scanned.series, catalog: scanned.catalog.unwrap() }
        );
    }
}
