//! The 0.11 document: one Section for the component, with a frame for each section of the handoff (Cover, Component Master,
//! Properties, Anatomy, States and Variants, Layouts, Tokens, Changelog), 1920 px wide with 256 px of room around and between
//! them. A rerun works section by section: a ticked section's frame is replaced in its place, an unticked one is left alone,
//! and the Section and the Component Master frame are never deleted (the original component lives in the Master).
//!
//! In this step only Cover, Component Master and Changelog have their content; the other five frames carry their title and a
//! line that says when they are made.

use super::*;

/// The frame's width, its padding and the useful width inside it.
pub const FRAME_W: f64 = 1920.0;
pub const FRAME_PAD: f64 = 48.0;
pub const USEFUL_W: f64 = FRAME_W - 2.0 * FRAME_PAD;
/// The room around the frames inside the Section, and between them.
pub const SECTION_PAD: f64 = 256.0;
pub const FRAME_GAP: f64 = 256.0;
/// The Section's width.
pub const SECTION_W: f64 = FRAME_W + 2.0 * SECTION_PAD;
/// The Section's background.
const SECTION_FILL: &str = "#161616";
/// The mark of the Section on the page (the value is the id of the component) and the mark of a frame's section.
pub const SECTION_MARK: &str = "handoff-report-section";
pub const NAME_MARK: &str = "handoff-section";

const CELL_PAD: [f64; 4] = [12.0, 16.0, 12.0, 16.0];
const TABLE_RADIUS: f64 = 8.0;
const APPROVALS_COLS: [f64; 6] = [240.0, 360.0, 220.0, 240.0, 240.0, 524.0];
const CHANGELOG_COLS: [f64; 3] = [200.0, 240.0, 1384.0];

// ---------------------------------------------------------------------------------------------
// The sections and which of them to make
// ---------------------------------------------------------------------------------------------

/// A section of the handoff, in the order of the frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sec {
    Cover,
    Master,
    Properties,
    Anatomy,
    Variants,
    Layouts,
    Tokens,
    Changelog,
}

impl Sec {
    pub const ALL: [Sec; 8] = [Sec::Cover, Sec::Master, Sec::Properties, Sec::Anatomy, Sec::Variants, Sec::Layouts, Sec::Tokens, Sec::Changelog];

    /// The value of the frame's `handoff-section` data, and the key in the `sections` parameter.
    pub fn key(self) -> &'static str {
        match self {
            Sec::Cover => "cover",
            Sec::Master => "master",
            Sec::Properties => "properties",
            Sec::Anatomy => "anatomy",
            Sec::Variants => "variants",
            Sec::Layouts => "layouts",
            Sec::Tokens => "tokens",
            Sec::Changelog => "changelog",
        }
    }

    /// The name of the panel's switch.
    pub fn control(self) -> &'static str {
        match self {
            Sec::Cover => "sec_cover",
            Sec::Master => "sec_master",
            Sec::Properties => "sec_properties",
            Sec::Anatomy => "sec_anatomy",
            Sec::Variants => "sec_variants",
            Sec::Layouts => "sec_layouts",
            Sec::Tokens => "sec_tokens",
            Sec::Changelog => "sec_changelog",
        }
    }

    pub fn title_key(self) -> Key {
        match self {
            Sec::Cover => Key::SecCover,
            Sec::Master => Key::SecMaster,
            Sec::Properties => Key::SecProperties,
            Sec::Anatomy => Key::SecAnatomy,
            Sec::Variants => Key::SecVariants,
            Sec::Layouts => Key::SecLayouts,
            Sec::Tokens => Key::SecTokens,
            Sec::Changelog => Key::SecChangelog,
        }
    }

    pub fn from_key(key: &str) -> Option<Sec> {
        Sec::ALL.into_iter().find(|s| s.key() == key)
    }

    fn index(self) -> usize {
        Sec::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// Which sections are ticked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Picks(pub [bool; 8]);

impl Picks {
    /// The first generation: everything.
    pub fn first_run() -> Picks {
        Picks([true; 8])
    }

    /// A rerun: Cover and Changelog hold text typed by hand, and the Master is already made; the rest is made again.
    pub fn rerun() -> Picks {
        let mut p = [true; 8];
        for s in [Sec::Cover, Sec::Master, Sec::Changelog] {
            p[s.index()] = false;
        }
        Picks(p)
    }

    pub fn get(&self, s: Sec) -> bool {
        self.0[s.index()]
    }

    pub fn set(&mut self, s: Sec, on: bool) {
        self.0[s.index()] = on;
    }

    pub fn any(&self) -> bool {
        self.0.iter().any(|b| *b)
    }

    pub fn to_json(self) -> Value {
        Value::Object(Sec::ALL.iter().map(|s| (s.key().to_string(), Value::Bool(self.get(*s)))).collect())
    }

    /// The picks a `sections` parameter says (a section it leaves out is not ticked); none when it is not an object.
    pub fn from_json(v: &Value) -> Option<Picks> {
        let o = v.as_object()?;
        let mut p = Picks([false; 8]);
        for s in Sec::ALL {
            p.set(s, o.get(s.key()).and_then(Value::as_bool).unwrap_or(false));
        }
        Some(p)
    }
}

/// Something the person should know after a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warn {
    /// The component is wider than the useful width of its Master frame.
    WideMaster,
    /// A loose documentation of version 0.10.0 was found and left as it was.
    LegacyDoc,
}

impl Warn {
    pub fn code(self) -> &'static str {
        match self {
            Warn::WideMaster => "wide_master",
            Warn::LegacyDoc => "legacy_doc",
        }
    }

    pub fn text(self, lang: Lang) -> &'static str {
        t(lang, match self {
            Warn::WideMaster => Key::PanelWarnWide,
            Warn::LegacyDoc => Key::PanelWarnLegacy,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Finding what is already there
// ---------------------------------------------------------------------------------------------

/// The id of the Section of `key` among the page's own children.
pub fn find_section(page: &PageInfo, key: &str) -> Option<u64> {
    page.children
        .iter()
        .rev()
        .find(|c| c.get("pluginData").and_then(|d| d.get(SECTION_MARK)).and_then(Value::as_str) == Some(key))
        .and_then(|c| c.get("id").and_then(Value::as_u64))
}

/// A loose documentation of version 0.10.0: a frame on the page with the old mark and no Section.
fn has_legacy_frame(page: &PageInfo, key: &str) -> bool {
    page.children.iter().any(|c| c.get("pluginData").and_then(|d| d.get(REPORT_KEY)).and_then(Value::as_str) == Some(key))
}

/// A frame of the Section: its id, its place in the Section (y) and its height.
#[derive(Clone, Debug, PartialEq)]
pub struct Existing {
    pub id: u64,
    pub sec: Sec,
    pub y: f64,
    pub h: f64,
}

/// The frames of a Section (one `node.get`): those with a known `handoff-section` mark, the first of each kind.
fn read_frames(lang: Lang, section: u64) -> Result<(Rect, Vec<Existing>)> {
    let node = host(lang, "node.get", &json!({"id": section, "fields": ["children", "pluginData", "absoluteBounds"]}))?;
    let sb = bounds(&node).unwrap_or_default();
    let mut out: Vec<Existing> = Vec::new();
    for c in node.get("children").and_then(Value::as_array).into_iter().flatten() {
        let Some(sec) = c.get("pluginData").and_then(|d| d.get(NAME_MARK)).and_then(Value::as_str).and_then(Sec::from_key) else { continue };
        let (Some(id), Some(b)) = (c.get("id").and_then(Value::as_u64), bounds(c)) else { continue };
        if out.iter().any(|e| e.sec == sec) {
            continue;
        }
        out.push(Existing { id, sec, y: b.y - sb.y, h: b.h });
    }
    Ok((sb, out))
}

/// Whether the component the selection stands for already has a Section on the current page (false when the selection says nothing).
pub fn section_exists(lang: Lang, selection: &[u64]) -> bool {
    let Ok(target) = resolve(lang, selection) else { return false };
    let Ok(page) = page_info() else { return false };
    find_section(&page, &target.key.to_string()).is_some()
}

// ---------------------------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------------------------

/// A frame of the Section, at `y` inside it (`x` is the room around): 1920 wide, padding 48, auto layout, hug.
fn new_frame(lang: Lang, section: u64, y: f64, name: &str, comp_key: &str, sec: Sec) -> Result<u64> {
    let created = host(
        lang,
        "frame.create",
        &json!({"parent": section, "x": SECTION_PAD, "y": y, "width": FRAME_W, "height": 200, "name": name, "select": false,
            "props": {"fill": theme::BG, "layout": "vertical", "gap": 32, "padding": FRAME_PAD, "crossAlign": "min", "sizingH": "fixed", "sizingV": "hug", "cornerRadius": TABLE_RADIUS, "clip": false,
                "pluginData": {REPORT_KEY: comp_key, NAME_MARK: sec.key()}}}),
    )?;
    id_of(lang, &created, "frame.create")
}

/// What a table cell holds: a text, a guide (grey, for the person to replace), or nothing.
enum Cv {
    Text(String),
    Guide(String),
    Blank,
}

fn cell(lang: Lang, grid: u64, col: usize, row: usize, fill: Option<&str>) -> Result<u64> {
    let mut f = FlowSpec::new("Cell", 100.0, "horizontal");
    f.padding = json!(CELL_PAD);
    f.fill = fill;
    f.gap = 8.0;
    f.extra = json!({"sizingH": "fill", "sizingV": "hug", "crossAlign": "center", "gridColumn": col, "gridRow": row, "gridAlignV": "min"});
    flow_frame(lang, grid, f)
}

fn cell_text(lang: Lang, parent: u64, s: &str, fill: &str, bold: bool) -> Result<u64> {
    let extra = if bold { json!({"sizingH": "fill", "textAutoResize": "height", "fontStyle": "Bold"}) } else { json!({"sizingH": "fill", "textAutoResize": "height"}) };
    text_with(lang, parent, s, LINE_SIZE, fill, "Text", extra)
}

fn separator(lang: Lang, grid: u64, row: usize, cols: usize) -> Result<()> {
    host(
        lang,
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 100, "height": 1, "parent": grid, "name": "Separator", "select": false,
            "props": {"fill": theme::BORDER, "gridColumn": 0, "gridRow": row, "gridColumnSpan": cols, "sizingH": "fill"}}),
    )?;
    Ok(())
}

/// A table of fixed columns: a header row, then the rows, each followed by a separator of 1 px (a grid with no row gap and
/// the border out of the layout, so the cells and the separators touch).
fn table(lang: Lang, frame: u64, name: &str, cols: &[f64], headers: &[String], rows: &[Vec<Cv>]) -> Result<u64> {
    let n = cols.len();
    let mut tracks: Vec<Value> = vec![json!("auto"), json!(1)];
    for _ in rows {
        tracks.push(json!("auto"));
        tracks.push(json!(1));
    }
    let created = host(
        lang,
        "frame.create",
        &json!({"parent": frame, "width": USEFUL_W, "height": 40, "name": name, "select": false,
            "props": {"layout": "grid", "gridColumns": n, "gridRows": 2 + 2 * rows.len(), "columnTracks": cols, "rowTracks": tracks, "gap": 0, "rowGap": 0, "strokesIncluded": false,
                "padding": 0, "sizingH": "fixed", "sizingV": "hug", "fill": theme::BG, "cornerRadius": TABLE_RADIUS, "stroke": theme::BORDER, "strokeWeight": 1, "strokeAlign": "inside", "clip": true}}),
    )?;
    let grid = id_of(lang, &created, "frame.create")?;
    for (col, h) in headers.iter().enumerate() {
        let c = cell(lang, grid, col, 0, Some(theme::BLOCK_FILL))?;
        cell_text(lang, c, h, theme::MUTED, true)?;
    }
    separator(lang, grid, 1, n)?;
    for (i, row) in rows.iter().enumerate() {
        let gr = 2 + 2 * i;
        for (col, v) in row.iter().enumerate() {
            match v {
                Cv::Blank => {}
                Cv::Text(s) => {
                    let c = cell(lang, grid, col, gr, None)?;
                    cell_text(lang, c, s, theme::TEXT, false)?;
                }
                Cv::Guide(s) => {
                    let c = cell(lang, grid, col, gr, None)?;
                    cell_text(lang, c, s, theme::MUTED_70, false)?;
                }
            }
        }
        separator(lang, grid, gr + 1, n)?;
    }
    Ok(grid)
}

/// What every frame needs to be drawn.
struct Ctx<'a> {
    lang: Lang,
    target: &'a Picked,
    version: String,
    status: String,
}

/// The frame's header: the component's name and the title of the section (the Cover adds the Version and Status chips).
fn header(c: &Ctx, frame: u64, sec: Sec) -> Result<()> {
    let lang = c.lang;
    let mut hd = FlowSpec::new("Header", USEFUL_W, "vertical");
    hd.gap = 12.0;
    let hd = flow_frame(lang, frame, hd)?;
    text_with(lang, hd, &c.target.title, theme::TITLE_SIZE, theme::TEXT, "Title", json!({}))?;
    if sec == Sec::Cover {
        let mut cr = FlowSpec::new("Chips", 20.0, "horizontal");
        cr.gap = 12.0;
        cr.extra = json!({"sizingH": "hug"});
        let cr = flow_frame(lang, hd, cr)?;
        let (version, status) = (shown_or_blank(lang, &c.version), shown_or_blank(lang, &c.status));
        for text in [tf(lang, Key::ChipVersion, &[("value", &version)]), tf(lang, Key::ChipStatus, &[("value", &status)])] {
            let chip = token_chip(lang, cr, json!([4, 12, 4, 12]), theme::BORDER, 1.0, json!({}))?;
            text_with(lang, chip, &text, LINE_SIZE, theme::TEXT, "Chip", json!({}))?;
        }
    }
    text_with(lang, hd, t(lang, sec.title_key()), theme::SECTION_SIZE, theme::MUTED, "Section title", json!({}))?;
    Ok(())
}

fn shown_or_blank(lang: Lang, s: &str) -> String {
    if s.trim().is_empty() { t(lang, Key::ToBeDefined).to_string() } else { s.trim().to_string() }
}

/// A label with the grey guide under it (the person writes over the guide on the canvas).
fn guide(lang: Lang, parent: u64, label: Key, hint: Key) -> Result<()> {
    let mut g = FlowSpec::new("Field", USEFUL_W, "vertical");
    g.gap = 4.0;
    let g = flow_frame(lang, parent, g)?;
    text_with(lang, g, t(lang, label), LINE_SIZE, theme::MUTED, "Label", json!({"fontStyle": "Bold"}))?;
    text_with(lang, g, t(lang, hint), LINE_SIZE, theme::MUTED_70, "Guide", json!({"width": USEFUL_W, "textAutoResize": "height"}))?;
    Ok(())
}

fn build_cover(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    header(c, frame, Sec::Cover)?;
    guide(lang, frame, Key::CoverDescription, Key::CoverGuideDescription)?;
    guide(lang, frame, Key::CoverLastDesigner, Key::CoverGuideDesigner)?;
    guide(lang, frame, Key::CoverUpdatedOn, Key::CoverGuideUpdated)?;
    text_with(lang, frame, t(lang, Key::CoverApprovals), theme::SECTION_SIZE, theme::TEXT, "Approvals title", json!({}))?;
    let headers: Vec<String> = [Key::ApprArea, Key::ApprOwner, Key::ApprState, Key::ApprVersion, Key::ApprDate, Key::ApprNote].iter().map(|k| t(lang, *k).to_string()).collect();
    let rows: Vec<Vec<Cv>> = [Key::RoleDesigner, Key::RoleEngineering, Key::RoleQa, Key::RoleWriters, Key::RolePo, Key::RolePm]
        .iter()
        .map(|role| {
            let g = || Cv::Guide(t(lang, Key::ApprGuide).to_string());
            vec![Cv::Text(t(lang, *role).to_string()), g(), Cv::Text(t(lang, Key::ApprPending).to_string()), g(), g(), g()]
        })
        .collect();
    table(lang, frame, "Approvals", &APPROVALS_COLS, &headers, &rows)?;
    Ok(())
}

fn build_changelog(c: &Ctx, frame: u64) -> Result<()> {
    let lang = c.lang;
    header(c, frame, Sec::Changelog)?;
    let headers: Vec<String> = [Key::ChangelogVersion, Key::ChangelogDate, Key::ChangelogChange].iter().map(|k| t(lang, *k).to_string()).collect();
    let rows = vec![vec![Cv::Guide(t(lang, Key::ChangelogGuideVersion).to_string()), Cv::Guide(t(lang, Key::ChangelogGuideDate).to_string()), Cv::Guide(t(lang, Key::ChangelogGuideChange).to_string())]];
    table(lang, frame, "Changelog", &CHANGELOG_COLS, &headers, &rows)?;
    Ok(())
}

/// A section that is not made yet: its title and a line that says when it will be.
fn build_reserved(c: &Ctx, frame: u64, sec: Sec) -> Result<()> {
    header(c, frame, sec)?;
    let step = if sec == Sec::Tokens { Key::ReservedC } else { Key::ReservedB };
    text_with(c.lang, frame, t(c.lang, step), LINE_SIZE, theme::MUTED_70, "Note", json!({}))?;
    Ok(())
}

/// The Component Master: the title, a line about it, the fixed warning, and the original component moved inside.
/// Returns the moved layer's id and whether it is wider than the frame's useful width.
fn build_master(c: &Ctx, frame: u64) -> Result<(u64, bool)> {
    let lang = c.lang;
    header(c, frame, Sec::Master)?;
    text_with(lang, frame, t(lang, Key::MasterDescription), LINE_SIZE, theme::MUTED, "Description", json!({"width": USEFUL_W, "textAutoResize": "height"}))?;
    text_with(lang, frame, t(lang, Key::MasterNote), LINE_SIZE, theme::WARN_LINE, "Warning", json!({"width": USEFUL_W, "textAutoResize": "height"}))?;
    let moved = c.target.holder;
    host(lang, "node.reparent", &json!({"ids": [moved], "parent": frame}))?;
    let node = host(lang, "node.get", &json!({"id": moved, "fields": ["absoluteBounds"]}))?;
    let wide = bounds(&node).is_some_and(|b| b.w > USEFUL_W + 0.5);
    Ok((moved, wide))
}

// ---------------------------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------------------------

/// Make or update the document: the Section of the selected component and the frames of the ticked sections.
pub fn generate(input: Input) -> Result<Value> {
    // 1. The page's settings come first: they give the language, even for an error.
    let page = page_info()?;
    let lang = page.settings.lang.as_deref().map_or(Lang::En, Lang::parse);
    if input.selection.is_empty() {
        return Err(t(lang, Key::ErrNoSelection).to_string());
    }
    let target = resolve(lang, &input.selection)?;
    // A component whose Section was deleted is gone with it: an instance may still name it.
    if host(lang, "node.get", &json!({"id": target.holder, "fields": ["type"]}))?.is_null() {
        return Err(t(lang, Key::ErrMainGone).to_string());
    }
    let comp_key = target.key.to_string();
    let name = target.title.clone();
    let ctx = Ctx { lang, target: &target, version: meta_text(input.str("version")), status: meta_text(input.str("status")) };
    // 2. Which Section and which sections. "New" makes a second Section beside everything; "replace" works on the one that is there.
    let section_id = if page.settings.rerun() == Rerun::Replace { find_section(&page, &comp_key) } else { None };
    let picks = input.params.get("sections").and_then(Picks::from_json).unwrap_or(if section_id.is_some() { Picks::rerun() } else { Picks::first_run() });
    if !picks.any() {
        return Err(t(lang, Key::PanelNoneChosen).to_string());
    }
    let mut warnings: Vec<Warn> = Vec::new();
    if section_id.is_none() && has_legacy_frame(&page, &comp_key) {
        warnings.push(Warn::LegacyDoc);
    }
    let right = page.children.iter().filter_map(bounds).map(Rect::right).fold(None, |r: Option<f64>, v| Some(r.map_or(v, |r| r.max(v))));
    let calls0 = HOST_CALLS.load(Ordering::Relaxed);
    // 3. The Section: the one that is there, or a new one right of everything (512 px beyond it, at y = 0).
    let (section, sx, sy, existing, created) = match section_id {
        Some(id) => {
            let (sb, frames) = read_frames(lang, id)?;
            (id, sb.x, sb.y, frames, false)
        }
        None => {
            let (x, y) = (right.map_or(0.0, |r| r + GAP_RIGHT), 0.0);
            let made = host(
                lang,
                "node.create",
                &json!({"type": "section", "name": tf(lang, Key::SectionName, &[("name", &name)]), "x": x, "y": y, "width": SECTION_W, "height": 400, "select": false,
                    "props": {"fill": SECTION_FILL, "pluginData": {SECTION_MARK: comp_key}}}),
            )?;
            (id_of(lang, &made, "node.create")?, x, y, Vec::new(), true)
        }
    };
    // 4. The frames, in order: a ticked section is made again in its place, an unticked one stays, a missing ticked one is made.
    let mut y = SECTION_PAD;
    let mut report: Vec<Value> = Vec::new();
    let mut master_moved: Option<u64> = None;
    for sec in Sec::ALL {
        let old = existing.iter().find(|e| e.sec == sec);
        let want = picks.get(sec);
        let (id, h, action): (u64, f64, &str) = match (old, sec, want) {
            // The Master is never deleted nor made again: it holds the original component.
            (Some(e), Sec::Master, _) => (e.id, e.h, "kept"),
            (Some(e), _, false) => (e.id, e.h, "kept"),
            (None, _, false) => continue,
            (old, _, true) => {
                if let Some(e) = old {
                    host(lang, "edit.delete", &json!({"ids": [e.id]}))?;
                }
                let frame_name = if sec == Sec::Master { tf(lang, Key::MasterFrameName, &[("name", &name)]) } else { tf(lang, Key::SectionFrameName, &[("title", t(lang, sec.title_key())), ("name", &name)]) };
                let frame = new_frame(lang, section, y, &frame_name, &comp_key, sec)?;
                match sec {
                    Sec::Cover => build_cover(&ctx, frame)?,
                    Sec::Changelog => build_changelog(&ctx, frame)?,
                    Sec::Master => {
                        let (moved, wide) = build_master(&ctx, frame)?;
                        master_moved = Some(moved);
                        if wide {
                            warnings.push(Warn::WideMaster);
                        }
                    }
                    other => build_reserved(&ctx, frame, other)?,
                }
                (frame, height_of(lang, frame)?, if old.is_some() { "replaced" } else { "created" })
            }
        };
        if action == "kept" {
            let current = existing.iter().find(|e| e.id == id).map_or(y, |e| e.y);
            if (current - y).abs() > 0.5 {
                host(lang, "node.set", &json!({"ids": [id], "props": {"y": y}}))?;
            }
        }
        report.push(json!({"key": sec.key(), "id": id, "y": y, "h": h, "action": action}));
        y += h + FRAME_GAP;
    }
    // 5. The Section is as tall as its frames and the room around them (`y` already holds the last gap).
    let section_h = y.max(2.0 * SECTION_PAD);
    host(lang, "node.set", &json!({"ids": [section], "props": {"height": section_h}}))?;
    let export_result = if created {
        match call("export.add", &json!({"ids": [section], "setting": {"size": "1x", "suffix": "", "format": "svg"}})) {
            Ok(_) => "ok".to_string(),
            Err(e) => e,
        }
    } else {
        "kept".to_string()
    };
    Ok(json!({
        "name": name, "frame": section, "section": section, "created": created, "rerun": !created, "x": sx, "y": sy, "frame_size": {"w": SECTION_W}, "section_size": {"w": SECTION_W, "h": section_h},
        "frames": report, "master_moved": master_moved, "warnings": warnings.iter().map(|w| w.code()).collect::<Vec<_>>(), "export": export_result,
        "picks": picks.to_json(), "lang": lang.code(), "meta": {"version": ctx.version, "status": ctx.status}, "component": target.key, "host_calls": HOST_CALLS.load(Ordering::Relaxed) - calls0,
    }))
}

/// "Go to Component Master": find the Master frame of the selected component, take the screen to it and select it.
/// Returns what it did, with the view before and after.
pub fn go_to_master(lang: Lang, selection: &[u64]) -> Result<Value> {
    let target = resolve(lang, selection)?;
    let page = page_info()?;
    let key = target.key.to_string();
    let Some(section) = find_section(&page, &key) else { return Ok(json!({"found": false, "reason": "no section"})) };
    let (_, frames) = read_frames(lang, section)?;
    let Some(master) = frames.iter().find(|e| e.sec == Sec::Master) else { return Ok(json!({"found": false, "reason": "no master frame"})) };
    let node = host(lang, "node.get", &json!({"id": master.id, "fields": ["absoluteBounds"]}))?;
    let Some(b) = bounds(&node) else { return Ok(json!({"found": false, "reason": "no bounds"})) };
    let before = host(lang, "view.get", &json!({}))?;
    let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64);
    let (vw, vh) = (num(&before, "vw").unwrap_or(0.0), num(&before, "vh").unwrap_or(0.0));
    let (zoom, cx, cy) = crate::panel::view_for(vw, vh, b.x, b.y, b.w);
    host(lang, "view.set", &json!({"zoom": zoom, "cx": cx, "cy": cy}))?;
    host(lang, "select.set", &json!({"ids": [master.id]}))?;
    let after = host(lang, "view.get", &json!({}))?;
    let view = |v: &Value| json!({"zoom": v.get("zoom"), "cx": v.get("cx"), "cy": v.get("cy")});
    Ok(json!({"found": true, "name": target.title, "frame": master.id, "bounds": {"x": b.x, "y": b.y, "w": b.w, "h": b.h}, "view_before": view(&before), "view_after": view(&after), "asked": {"zoom": zoom, "cx": cx, "cy": cy}}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sections_come_in_the_order_of_the_frames_with_their_own_keys() {
        let keys: Vec<&str> = Sec::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(keys, ["cover", "master", "properties", "anatomy", "variants", "layouts", "tokens", "changelog"]);
        for s in Sec::ALL {
            assert_eq!(Sec::from_key(s.key()), Some(s));
            assert!(s.control().starts_with("sec_"));
        }
        assert_eq!(Sec::from_key("nope"), None);
    }

    #[test]
    fn a_first_run_ticks_everything_and_a_rerun_leaves_the_manual_sections_and_the_master() {
        assert!(Sec::ALL.iter().all(|s| Picks::first_run().get(*s)));
        let r = Picks::rerun();
        for s in Sec::ALL {
            let manual_or_master = matches!(s, Sec::Cover | Sec::Master | Sec::Changelog);
            assert_eq!(r.get(s), !manual_or_master, "{s:?}");
        }
    }

    #[test]
    fn the_sections_parameter_round_trips_and_a_missing_key_is_not_ticked() {
        let p = Picks::rerun();
        assert_eq!(Picks::from_json(&p.to_json()), Some(p));
        let partial = Picks::from_json(&json!({"cover": true}));
        assert!(partial.is_some_and(|p| p.get(Sec::Cover) && !p.get(Sec::Tokens) && p.any()));
        assert_eq!(Picks::from_json(&json!("x")), None);
        assert!(Picks::from_json(&json!({})).is_some_and(|p| !p.any()));
    }

    #[test]
    fn the_columns_of_the_tables_fill_the_useful_width() {
        assert_eq!(USEFUL_W, 1824.0);
        assert_eq!(APPROVALS_COLS.iter().sum::<f64>(), USEFUL_W);
        assert_eq!(CHANGELOG_COLS.iter().sum::<f64>(), USEFUL_W);
        assert_eq!(SECTION_W, 2432.0);
    }

    #[test]
    fn the_section_is_found_by_its_mark_among_the_pages_own_children() {
        let page = PageInfo {
            id: 1,
            settings: Settings::default(),
            children: vec![json!({"id": 5, "pluginData": {"handoff-report-section": "10"}}), json!({"id": 6, "pluginData": {"handoff-report": "10"}}), json!({"id": 7, "pluginData": {"handoff-report-section": "11"}})],
        };
        assert_eq!(find_section(&page, "10"), Some(5));
        assert_eq!(find_section(&page, "11"), Some(7));
        assert_eq!(find_section(&page, "12"), None);
        assert!(has_legacy_frame(&page, "10"), "a loose frame with the old mark is a 0.10.0 documentation");
        assert!(!has_legacy_frame(&page, "11"));
    }
}
