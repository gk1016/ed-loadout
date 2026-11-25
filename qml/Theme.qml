pragma Singleton
import QtQuick

// Instrument, not webpage.
//
// The previous pass used a web design system - floating cards, 5px radii, drop
// shadows, generous padding, a centred content column. Rendered faithfully by
// Qt and it still read as a website, because it WAS one. This revision is built
// on the opposite rules:
//
//   * surfaces are DOCKED and share hairline edges; nothing floats
//   * radius is 0-2, not 5-6; shadows only on true overlays
//   * rows are 26px, not 52 - density is the point of an intelligence display
//   * colour encodes STATE, never decorates
//   * numerals are always monospaced and column-aligned
QtObject {
    id: theme

    // --- themes --------------------------------------------------------------
    //
    // TWO POLES, one token set (2026-10-17, the Deep Field rebrand). Every
    // consumer still says `Theme.pane` and `Theme.ink` - what changes is which
    // table those names read from. A view that names a colour by its ROLE
    // cannot be broken by a new palette, and a view that names one by its hue
    // is broken by the first one.
    //
    //   deepfield  the brand's dark pole: the web app's Deep Field tokens
    //              (base #04060B, card #0D121E, sand #E6C27A), the same set
    //              the iOS app adopted in TestFlight 94. THE DEFAULT.
    //   daylight   the brand's light pole (base #EEF1F7, sand #735819). Every
    //              text token is at least 4.5:1 on both grounds.
    //   system     follows the OS: Daylight when Windows is in light mode,
    //              Deep Field otherwise (Application.styleHints.colorScheme).
    //
    // The nine pre-brand poles (gunmetal, paper, carbon, dusk, cobra, night,
    // steel, midnight, the old daylight) are gone; AppSettings maps a stored
    // name to one of these two. The role names are unchanged, so nothing
    // outside this file moved - except the NEW `warn` tier below: caution
    // (stale, offline, unsynced, no fix, due, error notes) used to borrow the
    // accent, and the brand's sand now means brand only.
    property string name: "deepfield"
    readonly property bool system: name === "system"
    readonly property bool light: name === "daylight"
                               || (system && Application.styleHints.colorScheme === 1)   // Qt::ColorScheme::Light

    readonly property var poles: ({
        "deepfield": {
            chrome: "#04060B", base: "#070B14", pane: "#0D121E", header: "#121828",
            rowAlt: "#0A0F19", rowHot: "#141B2A", rowSel: "#1A2436",
            rule: "#1D2436", ruleHi: "#2A3449", grip: "#3A4660",
            ink: "#E9ECF5", dim: "#A3AABD", faint: "#8A91A7", ghost: "#5E6579",
            amber: "#E6C27A", amberHi: "#F2D59C",
            green: "#6FCF97", cyan: "#4FD6E0", red: "#F07A6C", warn: "#F2A65A",
            field: "#04060B", onAccent: "#1A1408"
        },
        "daylight": {
            chrome: "#E3E8F1", base: "#EEF1F7", pane: "#FFFFFF", header: "#E6EAF2",
            rowAlt: "#F6F8FB", rowHot: "#E9EDF4", rowSel: "#DCE4F0",
            rule: "#D6DCE8", ruleHi: "#B9C2D3", grip: "#98A2B6",
            ink: "#0B1020", dim: "#4A536B", faint: "#646C84", ghost: "#8A92A6",
            amber: "#735819", amberHi: "#57420F",
            green: "#1C7443", cyan: "#0A6E79", red: "#B13626", warn: "#C2410C",
            field: "#E3E8F1", onAccent: "#FFFFFF"
        }
    })

    readonly property var p: light ? poles["daylight"] : poles["deepfield"]

    // --- surfaces, in stacking order ---------------------------------------
    readonly property color chrome:  p.chrome    // command/status bars
    readonly property color base:    p.base      // workspace floor
    readonly property color pane:    p.pane      // docked pane body
    readonly property color header:  p.header    // pane + table headers
    readonly property color rowAlt:  p.rowAlt    // zebra
    readonly property color rowHot:  p.rowHot    // hover
    readonly property color rowSel:  p.rowSel    // selection

    // The inset surface: text fields, console wells, the sunken backgrounds a
    // readout sits in. Was the literal "#0b0e11" in twenty places, which is a
    // black box on a white panel the moment paper is selected.
    readonly property color field:   p.field
    // Text and glyphs drawn ON an accent fill, where neither ink nor the
    // surface colour is right because the surface under them is the accent.
    readonly property color onAccent: p.onAccent

    // --- structure ----------------------------------------------------------
    readonly property color rule:    p.rule      // hairline
    readonly property color ruleHi:  p.ruleHi    // emphasised divider
    readonly property color grip:    p.grip      // splitter handle

    // --- ink ----------------------------------------------------------------
    readonly property color ink:     p.ink
    readonly property color dim:     p.dim
    readonly property color faint:   p.faint
    readonly property color ghost:   p.ghost

    // --- state ---------------------------------------------------------------
    //
    // Brand names first (what the web and iOS apps call them); the old hue
    // names below are ALIASES kept so 48 views did not have to move. New code
    // says sand / ok / self / crit / warn.
    readonly property color sand:    p.amber     // the brand accent
    readonly property color sandHi:  p.amberHi
    readonly property color ok:      p.green     // nominal
    readonly property color self:    p.cyan      // informational / this seat
    readonly property color crit:    p.red       // critical
    readonly property color warn:    p.warn      // caution: stale, offline, unsynced, due, no fix
    readonly property color amber:   p.amber     // alias of sand (accent only - caution is `warn`)
    readonly property color amberHi: p.amberHi
    readonly property color green:   p.green
    readonly property color cyan:    p.cyan
    readonly property color red:     p.red
    function warnWash(a) { return Qt.rgba(warn.r, warn.g, warn.b, a) }

    // --- accent washes --------------------------------------------------------
    //
    // The tint under a selected row, a lit rail slot, a primary button. These
    // used to be literal `Qt.rgba(0.78, 0.59, 0.24, a)` calls - the gunmetal
    // amber, spelled out nineteen times - and every one of them would have
    // stayed amber-brown while midnight's accent went teal. Derived from the
    // live accent instead, so the wash is always the accent it belongs to.
    function wash(a)     { return Qt.rgba(amber.r,   amber.g,   amber.b,   a) }
    function washHi(a)   { return Qt.rgba(amberHi.r, amberHi.g, amberHi.b, a) }
    function dangerWash(a) { return Qt.rgba(red.r, red.g, red.b, a) }

    // Hover. White at 5-7% over a dark surface, which is invisible over a
    // white one - so on paper it inverts. The one place in this file where a
    // token has to know which way the surface runs.
    function hover(a) { return light ? Qt.rgba(0, 0, 0, a) : Qt.rgba(1, 1, 1, a) }

    // --- map canvas -----------------------------------------------------------
    //
    // THEME-INVARIANT, and the reason is physical rather than stylistic: the
    // galaxy is an additive starfield of 427,302 point sprites over a near
    // black clear colour, and additive light does not survive a white
    // background. The 3D surface was already pinned (GalaxyMap's clearColor);
    // these are for everything DRAWN ON IT - star names, region labels, the
    // pin badges - which until now read the chrome ink and would have gone
    // dark-on-black the moment paper was selected.
    //
    // Panes that FLOAT over the map (route, threats, coverage) are ordinary
    // panels with their own background and keep the chrome tokens: a light
    // panel over a dark map is the web's own answer and it reads correctly.
    // The values are the web's own `--map-*` set, which was picked against
    // this same near-black canvas.
    // One name per chrome name, so the swap in the map files is mechanical
    // and a reviewer can see at a glance that nothing was quietly re-toned:
    // mapInk is ink, mapDim is dim, and so on down.
    // The CANVAS itself follows the pole since 2026-08-26 - while staying
    // dark in every theme, because the additive-starfield physics has not
    // changed. Gunmetal keeps the original near-black, midnight goes
    // blue-black to match its pole, paper lifts to graphite so the map
    // visibly belongs to the light chrome without ever going light itself.
    // The ink set below stays pinned: it was picked against a near-black
    // canvas and all three of these still are one.
    // REVISED 2026-08-26: paper gets a genuinely LIGHT map canvas, by
    // operator demand - three rounds of lifting the dark greys still read as
    // "a void I cannot discern". The whole map family flips with it: dark ink
    // on a light ground. The instancing layer swaps its context palette
    // through StarInstancing.setLightCanvas, and the halo material drops from
    // additive to alpha blending - additive light on a light ground washes
    // white. Gunmetal and midnight keep the dark canvas and the pinned inks.
    // Deep Field (2026-10-17): the dark canvas IS the brand base and the
    // map inks are the pole's own inks; the light canvas is Daylight's map
    // ground (#E3E8F1) with Daylight's inks. One source, no hand-picked set.
    readonly property color mapBg: light ? "#E3E8F1" : "#04060B"

    readonly property color mapInk:      p.ink
    readonly property color mapDim:      p.dim
    readonly property color mapFaint:    p.faint
    readonly property color mapGhost:    p.ghost
    readonly property color mapRule:     light ? "#C4CCDA" : "#161C2B"
    readonly property color mapRuleHi:   light ? "#A9B3C6" : "#243049"
    readonly property color mapGrip:     p.grip
    readonly property color mapAccent:   p.amber
    readonly property color mapAccentHi: p.amberHi
    readonly property color mapCyan:     p.cyan
    readonly property color mapGreen:    p.green
    readonly property color mapRed:      p.red
    // The tooltip and readout backgrounds that sit ON the canvas. On the dark
    // canvases a light tip is a hole punched in the map; on the light canvas
    // the same logic runs the other way.
    readonly property color mapPane:     light ? "#F6F8FB" : "#0D121E"

    // --- settlement HUD (SiteView) ------------------------------------------
    //
    // The chrome drawn OVER the Godot site renderer. PINNED, not pole-driven:
    // the renderer's canvas is dark under every theme (unlike the map, whose
    // canvas flips with paper), so light-pole inks here would be unreadable.
    // One source for the seven greys SiteView used as 90-odd literals; the
    // settlement SYMBOLOGY (building types, door levels, threat marks) stays
    // literal in SiteView.legendRows on purpose - it is a legend, not a theme,
    // the same way the Powerplay glyph palette is pinned in Rust.
    readonly property color siteInkHi:   "#F2F4F7"   // headings, live values
    readonly property color siteInk:     "#C6CDD6"   // body text
    readonly property color siteDim:     "#8FA0B8"   // mono readouts
    readonly property color siteFaint:   "#6B7788"   // labels, inactive
    readonly property color siteRuleHi:  "#3A4452"   // field / button borders
    readonly property color siteRule:    "#2A3140"   // panel borders
    readonly property color siteRuleHot: "#5A6676"   // hovered border

    // --- chart series --------------------------------------------------------
    //
    // SIX slots, in FIXED order, and the state colours above are deliberately
    // not among them: amber, green and red mean caution, nominal and warning
    // everywhere else in this program, and a chart that spends them on "the
    // third faction" teaches the operator to stop reading them as state.
    //
    // VALIDATED, not chosen by eye, and validated PER CANVAS CLASS. The dark
    // six were checked against `pane` on both dark poles (#161b20 gunmetal,
    // #161c23 midnight - identical results) for the lightness band, a chroma
    // floor, adjacent-pair separation under protan and tritan colour
    // blindness, normal-vision separation and contrast: worst adjacent pair
    // 8.4 ΔE protan, 19.3 normal.
    //
    // The same six on white FAIL - they wash out - so paper gets its own
    // table at the same six HUES, darkened and re-validated against #ffffff:
    // worst adjacent pair 8.3 ΔE deutan, 15.7 normal. Same hue in the same
    // slot across all three themes, so a faction does not change colour when
    // the chrome does.
    //
    // Both CVD figures are in the 6-8+ floor band rather than comfortably
    // clear, so every chart using these MUST carry a second encoding -
    // direct labels on the lines, not a colour key on its own.
    //
    // A seventh series is not a seventh colour. It is excluded and counted.
    readonly property var seriesDark: [
        "#3987e5",  // blue
        "#d95926",  // orange
        "#199e70",  // aqua
        "#c98500",  // yellow
        "#d55181",  // magenta
        "#9085e9",  // violet
    ]
    readonly property var seriesLight: [
        "#1059b8",  // blue
        "#b0430f",  // orange
        "#12855c",  // aqua
        "#96650a",  // yellow
        "#b12356",  // magenta
        "#5a4bc4",  // violet
    ]
    readonly property var series: light ? seriesLight : seriesDark

    // --- type ---------------------------------------------------------------
    //
    // THE BRAND SET (2026-10-17), the same four the web and iOS apps carry,
    // all OFL and bundled in fonts/ (main.cpp registers every *.ttf there):
    //
    //   sans   IBM Plex Sans - the reading voice; what the program was drawn in.
    //   mono   IBM Plex Mono - numerals, readouts, everything column-aligned.
    //   head   Michroma - section heads, tab strips, kickers, the wordmark.
    //          Wide and all-caps by nature: it runs 1-2 px SMALLER than the
    //          label size (fHead) with 1.5 px tracking, as iOS's HumintSection.
    //   title  Jost Light - workspace and sheet titles.
    //   cond   Barlow Condensed - the rail captions and kv labels, unchanged.
    //
    // The B612 and Atkinson alternatives and the family picker are gone
    // (Graham, 2026-10-17: Deep Field only). `family` is kept as a no-op so a
    // stored ui/font value is accepted and ignored.
    property string family: "plex"
    readonly property string sans: "IBM Plex Sans"
    readonly property string mono: "IBM Plex Mono"
    readonly property string head: "Michroma"
    // Jost-Light.ttf names its family "Jost" (typographic) and "Jost Light"
    // (legacy); which one Qt registers depends on the platform database.
    readonly property string title: Qt.fontFamilies().indexOf("Jost") >= 0 ? "Jost" : "Jost Light"
    readonly property string cond: "Barlow Condensed"

    // --- posture -------------------------------------------------------------
    //
    // DESK is the dense watch floor this program was designed as: 26 px rows,
    // 12 px data, as much on screen at once as the eye can take.
    //
    // FLIGHT is the same program on a laptop beside the game, where the
    // commander is glancing at it between jumps with a stick in one hand. The
    // density that makes desk good makes flight unusable - so rows grow, type
    // grows, and hit targets grow with them. It is the SAME view either way:
    // inverting density is not the same as hiding half the information, and a
    // posture that answered fewer questions would just be a worse program.
    //
    // Written by the shell (Main.qml), which is the only thing that knows the
    // window width. Everything else reads the metrics below and follows.
    property string posture: "desk"
    readonly property bool flight: posture === "flight"
    // Set by the shell: a window wider than 3000 logical px (a 5120-wide
    // ultrawide at 100 %) - flight there takes the xl step, because the
    // pixel-exact type scale is the lever that does not blur (U-3).
    property bool wide: false

    // --- type scale (2026-08-29) ---------------------------------------------
    //
    // The lever UI SCALE is not: UI scale relaunches the app and scales the
    // map canvas and every hit target with it. The type scale changes ONLY
    // the font tokens and the row/rail metrics that hang off them - live, no
    // relaunch, map untouched.
    //
    //   compact    what desk was: 12 px data, 10 px labels, an 8 px rail.
    //              Kept for the operator who wants that density back.
    //   standard   13 / 11, a 10 px rail label on a 20 px glyph, 28 px rows.
    //              ~7% fewer rows on a 1080p table. THE DEFAULT: 13 px is
    //              where dim text clears APCA Lc 75 on the dark poles.
    //   large      14 / 12, 11 / 22 rail, 32 px rows.
    //   xl         16 / 14, 12 / 24 rail, 36 px rows (2026-10-17). What a
    //              49" 1440p at 100 % needs; flight on a wide window takes it.
    //
    // Posture maps onto it rather than fighting it: flight is always large
    // (the whole point of flight is bigger targets), desk takes the setting.
    // Secondary text goes to weight 500 above compact - at Lc ~60 a 400-weight
    // 11 px label is at the floor, 500 is comfortably over it.
    property string scale: "standard"
    readonly property string step: flight ? (wide ? "xl" : "large")
                                 : (scale === "compact" || scale === "large" || scale === "xl") ? scale
                                 : "standard"
    readonly property bool compact: step === "compact"
    readonly property bool xl:      step === "xl"
    readonly property bool large:   step === "large" || xl   // xl is large and then some
    readonly property int wSecondary: compact ? Font.Normal : Font.Medium

    readonly property int fLabel: xl ? 14 : large ? 12 : compact ? 10 : 11   // column heads, kv labels (cond)
    readonly property int fHead:  xl ? 12 : large ? 11 : compact ? 9 : 10    // section heads, tabs (Michroma)
    readonly property int fTitle: xl ? 22 : large ? 19 : compact ? 16 : 17   // workspace / sheet titles (Jost Light)
    readonly property int fMicro: xl ? 14 : large ? 12 : compact ? 10 : 11
    readonly property int fData:  xl ? 16 : large ? 14 : compact ? 12 : 13   // table cells
    readonly property int fBody:  xl ? 16 : large ? 14 : compact ? 12 : 13
    readonly property int fLead:  xl ? 20 : large ? 17 : compact ? 14 : 15

    // --- metrics -------------------------------------------------------------
    readonly property int row:       xl ? 36 : large ? 32 : compact ? 26 : 28
    readonly property int headerH:   xl ? 32 : large ? 28 : compact ? 24 : 26
    readonly property int barH:      xl ? 40 : large ? 36 : compact ? 30 : 32
    readonly property int statusH:   xl ? 32 : large ? 28 : compact ? 24 : 26
    readonly property int radius:    2

    // The tool rail is the one control that is ALWAYS on screen, and in flight
    // it is being hit with a hand that is also on a stick. It grows with the
    // step rather than staying at its compact size, because a 32 px target
    // reached blind is the one that gets missed - and an 8 px caption under
    // it was the single most-cited thing nobody could read.
    readonly property int railW:     xl ? 88 : large ? 80 : compact ? 64 : 72
    readonly property int railSlot:  xl ? 84 : large ? 76 : compact ? 60 : 68
    readonly property int railSlotH: xl ? 62 : large ? 56 : compact ? 44 : 50
    readonly property int railGlyph: xl ? 24 : large ? 22 : compact ? 15 : 20
    readonly property int railLabel: xl ? 12 : large ? 11 : compact ? 8 : 10

    // The tab strip is the second always-on control.
    readonly property int tabH:      xl ? 36 : large ? 32 : compact ? 26 : 28

    function scoreColor(v) {
        if (v === null || v === undefined) return ghost
        if (v >= 65) return green
        if (v >= 40) return amber
        return dim
    }

    // Powerplay control progress.
    //
    // pp_control_progress is a 0..1 fraction for 13,957 of the 13,968 rows that
    // have one. ELEVEN carry values like 12271.37 and 4294.93 - not a different
    // unit, just bad data in the snapshot. Rendering those as 1227136.6% is
    // absurd on sight; silently clamping them to 100% would be far worse,
    // because it would look correct. They are shown raw, marked, and coloured
    // as suspect.
    //
    // Lives here rather than in each view so the Systems table and the Anchors
    // table cannot disagree about what a control figure means.
    // PP2 control progress is the fraction of the CURRENT tier band, and
    // mid-week it legitimately runs outside 0..1: above 1.0 the system is
    // reinforced past the next threshold (tier-up applies at the Thursday
    // tick), below 0 it is undermined through the floor (tier-down at the
    // tick; max one tier change per cycle). -1.0 exactly is this bridge's
    // unknown sentinel. Far outside that envelope is upstream garbage
    // (12271.37-class artefacts) and stays marked raw rather than being
    // formatted into a story.
    function progressText(v) {
        if (v === null || v === undefined || v === -1.0) return "—"
        if (v > 2.0 || v < -0.5) return "?" + v.toFixed(2)
        if (v > 1.0) return (v * 100).toFixed(1) + "% · tier-up at tick"
        if (v < 0.0) return (v * 100).toFixed(1) + "% · tier-down at tick"
        return (v * 100).toFixed(1) + "%"
    }

    function progressTone(v) {
        if (v === null || v === undefined || v === -1.0) return ghost
        if (v > 2.0 || v < -0.5) return red
        if (v > 1.0 || v < 0.0) return amberHi
        return ink
    }

    // Minor-faction influence, as the engine reports it: a 0..1 fraction.
    //
    // Same discipline as progressText - the range is an ASSUMPTION about what
    // the number means, so anything outside it is shown raw and marked rather
    // than formatted into a plausible percentage.
    function influenceText(v) {
        if (v === null || v === undefined || v < 0) return "—"
        if (v > 1.0) return "?" + v.toFixed(2)
        return (v * 100).toFixed(1) + "%"
    }

    function influenceTone(v) {
        if (v === null || v === undefined || v < 0) return ghost
        if (v > 1.0) return red
        if (v >= 0.5) return amberHi
        return ink
    }

    // How old an observation is, from an ISO-8601 stamp the server sent.
    //
    // The AGE is the honest unit here: a wall-clock time would have to be
    // rendered in some timezone and would look equally current whether it
    // arrived a minute or a week ago.
    function ageText(iso) {
        if (!iso) return "—"
        const t = Date.parse(iso)
        if (isNaN(t)) return "—"
        const mins = (Date.now() - t) / 60000
        if (mins < 0) return "0m"          // server clock ahead of ours
        if (mins < 60) return Math.round(mins) + "m"
        if (mins < 48 * 60) return (mins / 60).toFixed(1) + "h"
        return Math.round(mins / 1440) + "d"
    }

    // --- unit formatting ------------------------------------------------------
    //
    // The ONLY place a view turns a domain number into display text. "QML
    // does no arithmetic" (CLAUDE.md) - these are the decode-and-format
    // helpers that rule allows, gathered here so the same quantity never
    // renders two ways and no view does the sum itself. Sentinels: a negative
    // or null domain value renders as a dash, never as 0.
    //
    // A fraction (0..1) as a percentage.
    function pct(frac, digits) {
        if (frac === null || frac === undefined) return "—"
        return (Number(frac) * 100).toFixed(digits === undefined ? 1 : digits) + "%"
    }
    // A fraction delta as percentage points ("+1.2pp" is the caller's sign).
    function pp(frac, digits) {
        return (Number(frac) * 100).toFixed(digits === undefined ? 1 : digits) + "pp"
    }
    // The map's compact BGS/PP payloads carry influence, margin and progress
    // as TENTHS of a percent in integers (/api/map/bgs, /api/map/pp).
    function tenthsPct(t, digits) {
        if (t === null || t === undefined || t < 0) return "—"
        return (Number(t) / 10).toFixed(digits === undefined ? 1 : digits) + "%"
    }
    // The long form of ageText for prose ("12 min ago"); "" when unknown or
    // in the future, so a caller can concatenate it without a dash.
    function ageLong(iso) {
        if (!iso) return ""
        const ms = Date.now() - Date.parse(iso)
        if (!isFinite(ms) || ms < 0) return ""
        const m = Math.floor(ms / 60000)
        if (m < 60) return m + " min ago"
        if (m < 2880) return Math.floor(m / 60) + " h ago"
        return Math.floor(m / 1440) + " d ago"
    }
    // An age already in hours (the engine's age_h / the journal's ageHours)
    // in ageText's units.
    function hoursText(h) {
        if (h === null || h === undefined || h < 0) return "—"
        return h < 1 ? Math.round(h * 60) + "m" : (h < 48 ? h.toFixed(1) + "h" : (h / 24).toFixed(1) + "d")
    }
    // Whole minutes since an ISO timestamp; -1 when unparsable.
    function ageMinutes(iso) {
        if (!iso) return -1
        const t = Date.parse(iso)
        if (isNaN(t)) return -1
        return Math.floor((Date.now() - t) / 60000)
    }
    // A countdown in seconds as "Nd Nh Nm".
    function dhm(seconds) {
        if (seconds === null || seconds === undefined || seconds < 0) return "—"
        const s = Math.floor(seconds)
        return Math.floor(s / 86400) + "d " + Math.floor((s % 86400) / 3600) + "h " + Math.floor((s % 3600) / 60) + "m"
    }
    // Light-seconds as the station lists show them.
    function ls(d) {
        if (d === null || d === undefined) return "-"
        if (d >= 10000) return (d / 1000).toFixed(0) + "k ls"
        return Number(d).toLocaleString(Qt.locale(), "f", 0) + " ls"
    }
    // Kilometres as whole metres.
    function kmToM(km) { return Math.round(Number(km) * 1000) }
    // Bytes as mebibytes, number only - the caller picks its unit label.
    function mib(bytes, digits) { return (Number(bytes) / 1048576).toFixed(digits === undefined ? 1 : digits) }

    // Anything older than a BGS tick is not telling you about today.
    function ageTone(iso) {
        if (!iso) return ghost
        const t = Date.parse(iso)
        if (isNaN(t)) return ghost
        const hours = (Date.now() - t) / 3600000
        if (hours > 26) return warn     // caution, not the brand accent
        return faint
    }

    function distColor(ly) {
        if (ly === null || ly === undefined) return ghost
        if (ly <= 50) return green
        if (ly <= 150) return amberHi
        return faint
    }
}
