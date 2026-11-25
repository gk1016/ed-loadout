import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// LOADOUT: one window. Command bar, tool rail, docked panes, status bar.
// Plan: claude/plans/2026-11-25-loadout-v0.3-desktop.md (Layout).
ApplicationWindow {
    id: win

    // --theme (harness) wins and is not saved; SETTINGS writes Core.theme.
    Binding { target: Theme; property: "name"; value: StartTheme !== "" ? StartTheme : Core.theme }
    // The dense step of FRAGO's Theme: 26 px rows, a 30 px command bar, a 24 px status bar.
    Binding { target: Theme; property: "scale"; value: "compact" }

    width: 1320
    height: 800
    minimumWidth: 1100
    minimumHeight: 620
    visible: true
    title: "LOADOUT"
    color: Theme.base

    readonly property var tabDefs: [
        { id: "home", label: "HOME", g: "⌂" },
        { id: "profiles", label: "PROFILES", g: "▤" },
        { id: "log", label: "LOG", g: "≡" },
        { id: "settings", label: "SETTINGS", g: "⚙" }
    ]
    property string tab: ["home", "profiles", "log", "settings"].indexOf(StartTab) >= 0 ? StartTab : "home"

    readonly property bool haveFacts: Core.detected.ok === true
    readonly property var facts: haveFacts ? Core.detected.facts : ({})

    // What APPLY needs: a profile, a plan that worked, an Options dir, no game.
    readonly property bool canApply: Core.selected !== "" && Core.plan.ok === true && Core.optionsOk && !Core.gameRunning
    function askApply() { if (canApply) confirm.open() }

    // A labelled value in the command bar, a hairline between two, and one in the status bar.
    component Seg: Row {
        property string label: ""
        property string value: ""
        property bool bad: false
        spacing: 7
        height: parent ? parent.height : 0
        Kicker { text: parent.label; height: parent.height }
        Text {
            text: parent.value
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            font.family: Theme.mono
            font.pixelSize: Theme.fData
            color: parent.bad ? Theme.warn : Theme.ink
        }
    }
    component Sep: Rectangle { width: 1; height: 14; color: Theme.ruleHi; anchors.verticalCenter: parent ? parent.verticalCenter : undefined }
    component Stat: Row {
        property string label: ""
        property string value: ""
        property color tone: Theme.dim
        spacing: 6
        Kicker { text: parent.label; height: parent.height; font.pixelSize: Theme.fLabel }
        Text {
            text: parent.value
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            font.family: Theme.mono
            font.pixelSize: Theme.fMicro
            color: parent.tone
        }
    }

    // ---- command bar
    header: Rectangle {
        height: Theme.barH
        color: Theme.chrome

        Row {
            anchors { left: parent.left; leftMargin: 11; top: parent.top; bottom: parent.bottom }
            spacing: 14
            Row {
                spacing: 8
                height: parent.height
                Mark { size: 18; anchors.verticalCenter: parent.verticalCenter }
                Text {
                    text: "LOADOUT"
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    font.family: Theme.cond
                    font.pixelSize: 16
                    font.weight: Font.Bold
                    font.letterSpacing: 2.5
                    color: Theme.ink
                }
            }
            Sep {}
            Seg { label: "MACHINE"; value: win.haveFacts ? win.facts.hostname : "not detected"; bad: !win.haveFacts }
            Sep {}
            Seg { label: "GPU"; value: win.haveFacts ? win.facts.gpu_name : "-" }
            Sep {}
            Seg { label: "PANEL"; value: win.haveFacts ? win.facts.screen_w + "x" + win.facts.screen_h + "@" + win.facts.refresh_hz : "-" }
            Sep {}
            Seg { label: "TIER"; value: win.haveFacts ? Core.detected.tier : "-" }
        }
        Text {
            anchors { right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter }
            text: "v" + BuildInfo.version
            font.family: Theme.mono
            font.pixelSize: Theme.fData
            color: Theme.dim
        }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.rule }
    }

    // ---- status bar
    footer: Rectangle {
        height: Theme.statusH
        color: Theme.chrome
        Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: Theme.rule }

        RowLayout {
            anchors { fill: parent; leftMargin: 11; rightMargin: 12 }
            spacing: 16
            Kicker { text: "OPTIONS"; font.pixelSize: Theme.fLabel; Layout.fillHeight: true }
            Text {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.leftMargin: -10
                verticalAlignment: Text.AlignVCenter
                text: Core.optionsDir + (Core.optionsOk ? "" : "   NOT FOUND")
                elide: Text.ElideMiddle
                font.family: Theme.mono
                font.pixelSize: Theme.fMicro
                color: Core.optionsOk ? Theme.dim : Theme.crit
            }
            Stat { label: "LAST BACKUP"; value: Core.lastBackup !== "" ? Core.lastBackup : "none"; Layout.fillHeight: true }
            Stat {
                label: "GAME"
                value: Core.gameRunning ? "RUNNING" : "not running"
                tone: Core.gameRunning ? Theme.crit : Theme.dim
                Layout.fillHeight: true
            }
            Stat { label: "BUILD"; value: BuildInfo.sha; Layout.fillHeight: true }
        }
    }

    // ---- tool rail + the docked panes
    Rail {
        id: rail
        anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
        defs: win.tabDefs
        current: win.tab
        onChosen: (id) => win.tab = id
    }
    Item {
        anchors { left: rail.right; right: parent.right; top: parent.top; bottom: parent.bottom }

        SplitView {
            anchors.fill: parent
            visible: win.tab === "home"
            orientation: Qt.Horizontal
            // A hairline with a wider grab area.
            handle: Rectangle {
                id: grip
                implicitWidth: 1
                color: SplitHandle.pressed || SplitHandle.hovered ? Theme.grip : Theme.rule
                containmentMask: Item { x: -3; width: 7; height: grip.height }
            }
            ProfilesPage {
                SplitView.preferredWidth: 430
                SplitView.minimumWidth: 340
                SplitView.maximumWidth: 640
            }
            HomePage {
                SplitView.fillWidth: true
                SplitView.minimumWidth: 560
                canApply: win.canApply
                onApplyAsked: win.askApply()
            }
        }
        ProfilesPage { anchors.fill: parent; visible: win.tab === "profiles"; detail: true }
        LogPage { anchors.fill: parent; visible: win.tab === "log" }
        SettingsPage {
            anchors.fill: parent
            visible: win.tab === "settings"
            theme: Theme.name
            onThemeChosen: (name) => Core.theme = name
        }
    }

    ConfirmApply {
        id: confirm
        onAccepted: Core.apply()
    }

    // ---- harness (ship.ps1's scripted run, shots.ps1): the same controls a click reaches.
    property var script: StartScript
    function step() {
        if (script.length === 0) { stepper.stop(); return }
        const s = script[0]
        script = script.slice(1)
        if (s === "capture") {
            const ok = Core.captureThisMachine()
            console.info("script: capture ok=" + ok + " profiles=" + Core.profiles.length + " selected=" + (Core.selectedProfile.name || ""))
        } else if (s === "apply") {
            console.info("script: apply enabled=" + win.canApply)
            win.askApply()
            console.info("script: confirm open=" + confirm.visible + " files=" + confirm.files.length + " backup under " + confirm.backupRoot
                         + " header=\"" + confirm.headline + "\"")
            script = ["accept"].concat(script)
        } else if (s === "accept") {
            if (confirm.visible) {
                confirm.accept()
                console.info("script: applied; " + Core.notice + " log lines=" + Core.log.length)
            } else {
                console.info("script: nothing applied (the confirm is not open)")
            }
        } else if (s === "restore") {
            const ok = Core.restoreLast()
            console.info("script: restore ok=" + ok + "; " + Core.notice)
        } else if (s === "log") {
            win.tab = "log"
        }
    }
    Timer { id: stepper; interval: 600; repeat: true; running: win.script.length > 0; onTriggered: win.step() }
    Component.onCompleted: {
        if (StartSelect !== "") {
            const hit = Core.profiles.filter(function (p) { return p.name === StartSelect })
            if (hit.length > 0) Core.select(hit[0].path)
        }
        console.info("home: detected=" + win.haveFacts + " profiles=" + Core.profiles.length
                     + " selected=" + (Core.selectedProfile.name || "none") + " game=" + Core.gameRunning
                     + " apply enabled=" + win.canApply)
        if (StartConfirm) Qt.callLater(win.askApply)
    }
}
