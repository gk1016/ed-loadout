import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// HOME: what this machine is (DETECTED), what the selected profile would
// write here (WILL APPLY), and the three actions.
Rectangle {
    id: page
    signal applyAsked()
    // Set by the shell: a profile, a plan that worked, an Options dir, no game.
    property bool canApply: false

    color: Theme.pane

    readonly property bool haveFacts: Core.detected.ok === true
    readonly property var facts: haveFacts ? Core.detected.facts : ({})
    readonly property var plan: Core.plan
    readonly property bool planned: plan.ok === true
    readonly property string profileName: Core.selectedProfile.name || ""

    component Rule: Rectangle { width: parent ? parent.width : 0; height: 1; color: Theme.rule }

    // One label / value row.
    component Fact: Item {
        id: fact
        property string label: ""
        property string value: ""
        property color tone: Theme.ink
        readonly property int labelW: 150
        default property alias extras: slot.data
        width: parent ? parent.width : 0
        height: Theme.row
        Kicker { x: 10; width: fact.labelW - 10; height: parent.height; text: fact.label }
        Text {
            x: fact.labelW
            width: parent.width - x - 10
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: fact.value
            elide: Text.ElideRight
            font.family: Theme.mono
            font.pixelSize: Theme.fData
            color: fact.tone
        }
        Row { id: slot; x: fact.labelW; height: parent.height; spacing: 6 }
        Rule { anchors.bottom: parent.bottom }
    }

    // A full-width line of plain text (the empty and error states).
    component Line: Item {
        id: line
        property string text: ""
        property color tone: Theme.dim
        width: parent ? parent.width : 0
        height: Theme.row
        Text {
            x: 10
            width: parent.width - 20
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: line.text
            elide: Text.ElideRight
            font.family: Theme.sans
            font.pixelSize: Theme.fBody
            color: line.tone
        }
        Rule { anchors.bottom: parent.bottom }
    }

    // One table cell.
    component Cell: Text {
        height: parent ? parent.height : 0
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        font.family: Theme.mono
        font.pixelSize: Theme.fData
        color: Theme.ink
    }

    // Column heads of a table.
    component Heads: Rectangle {
        id: heads
        property var cols: []      // [{t, w}] - w 0 = the rest
        width: parent ? parent.width : 0
        height: Theme.headerH
        color: Theme.rowAlt
        Row {
            x: 10
            height: parent.height
            Repeater {
                model: heads.cols
                delegate: Kicker {
                    required property var modelData
                    width: modelData.w
                    height: heads.height
                    text: modelData.t
                    font.pixelSize: Theme.fLabel
                }
            }
        }
        Rule { anchors.bottom: parent.bottom }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Flickable {
            id: flick
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: body.height
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            Column {
                id: body
                width: flick.width

                // ---- DETECTED
                PaneHead {
                    width: parent.width
                    title: "DETECTED"
                    note: page.haveFacts ? "this machine, read just now" : (Core.detected.error || "")
                    noteBad: !page.haveFacts
                }
                Line {
                    visible: !page.haveFacts
                    text: "This machine could not be detected. A profile still applies as captured; nothing is derived."
                    tone: Theme.warn
                }
                Column {
                    width: parent.width
                    visible: page.haveFacts
                    Fact { label: "HOSTNAME"; value: page.facts.hostname || "" }
                    Fact { label: "PANEL"; value: (page.facts.screen_w || 0) + " x " + (page.facts.screen_h || 0) }
                    Fact { label: "REFRESH"; value: (page.facts.refresh_hz || 0) + " Hz" }
                    Fact { label: "GPU"; value: page.facts.gpu_name || "" }
                    Fact { label: "VRAM"; value: (page.facts.gpu_vram_mb || 0) + " MB" }
                    Fact { label: "TIER"; value: Core.detected.tier || "" }
                    Fact {
                        label: "CONTROLLERS"
                        value: (page.facts.devices || []).length === 0 ? "none connected" : ""
                        tone: Theme.dim
                        Repeater {
                            model: page.facts.devices || []
                            delegate: Rectangle {
                                required property string modelData
                                anchors.verticalCenter: parent.verticalCenter
                                width: chip.implicitWidth + 12
                                height: Theme.row - 8
                                radius: Theme.radius
                                color: "transparent"
                                border.width: 1
                                border.color: Theme.ruleHi
                                Text {
                                    id: chip
                                    anchors.centerIn: parent
                                    text: modelData
                                    font.family: Theme.mono
                                    font.pixelSize: Theme.fMicro
                                    color: Theme.ink
                                }
                            }
                        }
                    }
                }

                // ---- WILL APPLY
                PaneHead {
                    width: parent.width
                    title: "WILL APPLY"
                    note: page.profileName !== "" ? page.profileName + "  ->  " + Core.optionsDir : ""
                }
                Line {
                    visible: Core.profiles.length === 0
                    text: "No profile yet. CAPTURE THIS MACHINE or IMPORT FOLDER in the PROFILES pane."
                }
                Line {
                    visible: Core.profiles.length > 0 && Core.selected === ""
                    text: "No profile selected. Pick one in the PROFILES pane."
                }
                Line {
                    visible: Core.selected !== "" && !page.planned
                    text: page.plan.error || ""
                    tone: Theme.crit
                }
                Column {
                    width: parent.width
                    visible: page.planned

                    Heads { cols: [{ t: "KEY", w: 150 }, { t: "VALUE", w: 80 }, { t: "SOURCE", w: 230 }, { t: "REASON", w: 300 }] }
                    Line {
                        visible: (page.plan.rows || []).length === 0
                        text: "No derived or overridden keys: the profile's files are written as captured."
                    }
                    Repeater {
                        model: page.plan.rows || []
                        delegate: Item {
                            id: r
                            required property var modelData
                            readonly property bool mine: modelData.source !== "derived"
                            width: body.width
                            height: Theme.row
                            Cell { x: 10; width: 144; text: r.modelData.key }
                            Cell { x: 160; width: 74; text: r.modelData.value }
                            Cell { x: 240; width: 224; text: r.modelData.source; color: Theme.dim }
                            Cell {
                                x: 470
                                width: r.width - x - 10
                                text: r.modelData.reason
                                font.family: Theme.sans
                                color: Theme.dim
                            }
                            HoverHandler { id: rh }
                            ToolTip.visible: rh.hovered && r.modelData.reason.length > 60
                            ToolTip.delay: 500
                            ToolTip.text: r.modelData.reason
                            Rule { anchors.bottom: parent.bottom }
                        }
                    }

                    Heads { cols: [{ t: "FILE TARGET", w: 460 }, { t: "WRITE", w: 130 }, { t: "ELEMENT EDITS", w: 200 }] }
                    Line {
                        visible: (page.plan.files || []).length === 0
                        text: "This profile writes no files."
                    }
                    Repeater {
                        model: page.plan.files || []
                        delegate: Item {
                            id: f
                            required property var modelData
                            width: body.width
                            height: Theme.row
                            Cell { x: 10; width: 454; text: f.modelData.rel; elide: Text.ElideMiddle }
                            Cell {
                                x: 470; width: 124
                                text: !f.modelData.exists ? "new file" : f.modelData.changed ? "overwrite" : "same bytes"
                                color: Theme.dim
                            }
                            Cell { x: 600; width: f.width - x - 10; text: f.modelData.edits > 0 ? f.modelData.edits : "-"; color: Theme.dim }
                            Rule { anchors.bottom: parent.bottom }
                        }
                    }

                    // Bound devices from the profile that are not connected here.
                    Repeater {
                        model: page.plan.unconnected || []
                        delegate: Item {
                            id: u
                            required property var modelData
                            width: body.width
                            height: Theme.row
                            Kicker { x: 10; width: 240; height: parent.height; text: "BOUND DEVICE NOT CONNECTED"; color: Theme.warn }
                            Text {
                                x: 250
                                width: parent.width - x - 10
                                height: parent.height
                                verticalAlignment: Text.AlignVCenter
                                text: u.modelData.guid + "   " + u.modelData.bindings + (u.modelData.bindings === 1 ? " binding" : " bindings")
                                font.family: Theme.mono
                                font.pixelSize: Theme.fData
                                color: Theme.warn
                            }
                            Rule { anchors.bottom: parent.bottom }
                        }
                    }
                    Line {
                        visible: page.plan.facts === false
                        text: "Bound devices were not checked: this machine's controllers could not be read."
                        tone: Theme.warn
                    }
                }
            }
        }

        // ---- action bar
        Rectangle {
            Layout.fillWidth: true
            height: Theme.row + 12
            color: Theme.header
            Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: Theme.rule }
            Row {
                id: actions
                x: 10
                anchors.verticalCenter: parent.verticalCenter
                spacing: 8
                Btn { label: "DETECT AGAIN"; onClicked: Core.detectAgain() }
                Btn {
                    label: page.profileName !== "" ? "APPLY " + page.profileName.toUpperCase() : "APPLY"
                    primary: true
                    live: page.canApply
                    onClicked: page.applyAsked()
                }
                Btn {
                    label: "RESTORE LAST BACKUP"
                    live: Core.lastBackup !== "" && !Core.gameRunning
                    onClicked: Core.restoreLast()
                }
            }
            Text {
                anchors { left: actions.right; leftMargin: 14; right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter }
                horizontalAlignment: Text.AlignRight
                elide: Text.ElideLeft
                text: Core.gameRunning ? "Elite Dangerous is running: APPLY and RESTORE are off until it is closed."
                    : !Core.optionsOk ? "Options dir not found. Set it in SETTINGS."
                    : Core.notice
                font.family: Theme.sans
                font.pixelSize: Theme.fBody
                color: Core.gameRunning || !Core.optionsOk || Core.noticeBad ? Theme.crit : Theme.dim
            }
        }
    }
}
