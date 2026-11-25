import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The APPLY confirm: every file about to be written and where the backup
// goes. Nothing is written until APPLY NOW.
Popup {
    id: sheet
    signal accepted()

    readonly property var plan: Core.plan
    readonly property var files: plan.files || []
    readonly property var missing: plan.unconnected || []
    readonly property string backupRoot: plan.backup_root || ""
    // Files actually written: new or with different bytes. "Same bytes" rows are not writes.
    readonly property int writes: files.filter(function (f) { return !f.exists || f.changed }).length
    readonly property int unchanged: files.length - writes
    readonly property string headline: writes + (writes === 1 ? " file" : " files") + " will be written"
                                        + (unchanged > 0 ? ", " + unchanged + " unchanged" : "")
    function accept() { sheet.close(); sheet.accepted() }

    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape
    anchors.centerIn: Overlay.overlay
    width: Math.min(820, Overlay.overlay ? Overlay.overlay.width - 80 : 820)
    padding: 0
    // A docked sheet: hairline, radius 2, no shadow.
    background: Rectangle { color: Theme.pane; radius: Theme.radius; border.width: 1; border.color: Theme.ruleHi }
    Overlay.modal: Rectangle { color: Theme.light ? Qt.rgba(0.04, 0.06, 0.12, 0.35) : Qt.rgba(0, 0, 0, 0.6) }

    component Rule: Rectangle { width: parent ? parent.width : 0; height: 1; color: Theme.rule }
    component Cell: Text {
        height: parent ? parent.height : 0
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideMiddle
        font.family: Theme.mono
        font.pixelSize: Theme.fData
        color: Theme.ink
    }
    component Pair: Item {
        id: pair
        property string label: ""
        property string value: ""
        width: parent ? parent.width : 0
        height: Theme.row
        Kicker { x: 10; width: 130; height: parent.height; text: pair.label }
        Text {
            x: 150
            width: parent.width - 160
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: pair.value
            elide: Text.ElideMiddle
            font.family: Theme.mono
            font.pixelSize: Theme.fData
            color: Theme.ink
        }
        Rule { anchors.bottom: parent.bottom }
    }

    contentItem: Column {
        width: sheet.width
        padding: 1

        PaneHead {
            width: sheet.width - 2
            title: "APPLY " + (Core.selectedProfile.name || "").toUpperCase()
            note: sheet.headline
        }
        Pair { width: sheet.width - 2; label: "OPTIONS DIR"; value: Core.optionsDir }
        Pair { width: sheet.width - 2; label: "BACKUP FOLDER"; value: sheet.backupRoot + "\\<time of this apply>" }

        Rectangle {
            width: sheet.width - 2
            height: Theme.headerH
            color: Theme.rowAlt
            Kicker { x: 10; width: 430; height: parent.height; text: "FILE"; font.pixelSize: Theme.fLabel }
            Kicker { x: 450; width: 150; height: parent.height; text: "WRITE"; font.pixelSize: Theme.fLabel }
            Kicker { x: 610; width: 150; height: parent.height; text: "BACKED UP"; font.pixelSize: Theme.fLabel }
            Rule { anchors.bottom: parent.bottom }
        }
        ListView {
            id: list
            width: sheet.width - 2
            height: Math.min(contentHeight, Theme.row * 11)
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: sheet.files
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            delegate: Item {
                id: row
                required property var modelData
                width: list.width
                height: Theme.row
                Cell { x: 10; width: 430; text: row.modelData.rel }
                Cell {
                    x: 450; width: 150
                    text: !row.modelData.exists ? "new file" : row.modelData.changed ? "overwrite" : "same bytes"
                    color: Theme.dim
                }
                Cell { x: 610; width: 150; text: row.modelData.exists ? "yes" : "nothing to back up"; color: Theme.dim }
                Rule { anchors.bottom: parent.bottom }
            }
        }
        Repeater {
            model: sheet.missing
            delegate: Item {
                id: miss
                required property var modelData
                width: sheet.width - 2
                height: Theme.row
                Kicker { x: 10; width: 240; height: parent.height; text: "BOUND DEVICE NOT CONNECTED"; color: Theme.crit }
                Text {
                    x: 250
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    text: miss.modelData.guid + "   " + miss.modelData.bindings + (miss.modelData.bindings === 1 ? " binding" : " bindings")
                    font.family: Theme.mono
                    font.pixelSize: Theme.fData
                    color: Theme.crit
                }
                Rule { anchors.bottom: parent.bottom }
            }
        }
        Rectangle {
            width: sheet.width - 2
            height: Theme.row + 12
            color: Theme.header
            Text {
                anchors { left: parent.left; leftMargin: 10; right: buttons.left; rightMargin: 12; verticalCenter: parent.verticalCenter }
                elide: Text.ElideRight
                text: Core.gameRunning ? "Elite Dangerous is running: close it first."
                                       : "RESTORE LAST BACKUP on HOME undoes this apply."
                font.family: Theme.sans
                font.pixelSize: Theme.fBody
                color: Core.gameRunning ? Theme.crit : Theme.dim
            }
            Row {
                id: buttons
                anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                spacing: 8
                Btn { label: "CANCEL"; onClicked: sheet.close() }
                Btn { label: "APPLY NOW"; primary: true; live: !Core.gameRunning && sheet.plan.ok === true; onClicked: sheet.accept() }
            }
        }
    }
}
