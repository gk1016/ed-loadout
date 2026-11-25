import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// PROFILES: the captured profiles in the profiles folder. Docked on the left
// of HOME; on the PROFILES rail page (`detail`) with the selected profile's
// files beside it.
Rectangle {
    id: page
    property bool detail: false

    color: Theme.pane

    readonly property var sel: Core.selectedProfile
    function shortTime(iso) { return iso ? iso.replace("T", " ").slice(0, 16) + "Z" : "-" }

    component Rule: Rectangle { width: parent ? parent.width : 0; height: 1; color: Theme.rule }
    component Cell: Text {
        height: parent ? parent.height : 0
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        font.family: Theme.mono
        font.pixelSize: Theme.fData
        color: Theme.ink
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // ---- the list
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredWidth: 430
            spacing: 0

            PaneHead {
                Layout.fillWidth: true
                title: "PROFILES"
                note: Core.profilesDir
            }
            Rectangle {
                Layout.fillWidth: true
                height: Theme.headerH
                color: Theme.rowAlt
                Kicker { x: 10; width: list.nameW; height: parent.height; text: "NAME"; font.pixelSize: Theme.fLabel }
                Kicker { x: 10 + list.nameW; width: list.srcW; height: parent.height; text: "SOURCE MACHINE"; font.pixelSize: Theme.fLabel }
                Kicker { x: 10 + list.nameW + list.srcW; width: list.capW; height: parent.height; text: "CAPTURED"; font.pixelSize: Theme.fLabel }
                Kicker { x: 10 + list.nameW + list.srcW + list.capW; width: 50; height: parent.height; text: "FILES"; font.pixelSize: Theme.fLabel }
                Rule { anchors.bottom: parent.bottom }
            }
            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                model: Core.profiles
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                // NAME and SOURCE share what CAPTURED and FILES leave.
                readonly property int capW: 132
                readonly property int nameW: Math.max(110, Math.round((width - 20 - capW - 50) * 0.52))
                readonly property int srcW: Math.max(90, width - 20 - capW - 50 - nameW)

                delegate: Rectangle {
                    id: row
                    required property var modelData
                    readonly property bool picked: modelData.path === Core.selected
                    width: list.width
                    height: Theme.row
                    color: picked ? Theme.rowSel : (hov.hovered ? Theme.rowHot : "transparent")
                    Rectangle { visible: row.picked; width: 2; height: parent.height; color: Theme.sand }
                    Cell { x: 10; width: list.nameW - 6; text: row.modelData.name }
                    Cell {
                        x: 10 + list.nameW; width: list.srcW - 6
                        text: row.modelData.error !== "" ? row.modelData.error : row.modelData.source_machine
                        color: row.modelData.error !== "" ? Theme.crit : Theme.dim
                    }
                    Cell { x: 10 + list.nameW + list.srcW; width: list.capW - 6; text: page.shortTime(row.modelData.captured_at); color: Theme.dim }
                    Cell { x: 10 + list.nameW + list.srcW + list.capW; width: 44; text: row.modelData.files.length; color: Theme.dim }
                    Rule { anchors.bottom: parent.bottom }
                    HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: Core.select(row.modelData.path) }
                }

                Text {
                    visible: list.count === 0
                    x: 10
                    width: parent.width - 20
                    height: Theme.row
                    verticalAlignment: Text.AlignVCenter
                    text: "No profile yet. Capture this machine or import a folder."
                    elide: Text.ElideRight
                    font.family: Theme.sans
                    font.pixelSize: Theme.fBody
                    color: Theme.dim
                }
            }
            // ---- the ruled action bar
            Rectangle {
                Layout.fillWidth: true
                height: Theme.row + 12
                color: Theme.header
                Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: Theme.rule }
                Row {
                    x: 10
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 8
                    Btn { label: "CAPTURE THIS MACHINE"; live: Core.optionsOk; onClicked: Core.captureThisMachine() }
                    Btn { label: "IMPORT FOLDER"; onClicked: Core.importFolder() }
                }
            }
        }

        // ---- the selected profile's files (PROFILES page only)
        Rectangle { visible: page.detail; Layout.fillHeight: true; width: 1; color: Theme.rule }
        ColumnLayout {
            visible: page.detail
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredWidth: 560
            spacing: 0

            PaneHead {
                Layout.fillWidth: true
                title: "FILES IN PROFILE"
                note: page.sel.name ? page.sel.name + "   " + page.sel.path : ""
            }
            Rectangle {
                Layout.fillWidth: true
                height: Theme.headerH
                color: Theme.rowAlt
                Kicker { x: 10; width: 330; height: parent.height; text: "FILE"; font.pixelSize: Theme.fLabel }
                Kicker { x: 350; width: 200; height: parent.height; text: "SHA-256"; font.pixelSize: Theme.fLabel }
                Rule { anchors.bottom: parent.bottom }
            }
            ListView {
                id: files
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                model: page.sel.files || []
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                delegate: Item {
                    id: frow
                    required property var modelData
                    width: files.width
                    height: Theme.row
                    Cell { x: 10; width: 334; text: frow.modelData.rel; elide: Text.ElideMiddle }
                    Cell { x: 350; width: parent.width - 360; text: frow.modelData.sha256; color: Theme.dim }
                    Rule { anchors.bottom: parent.bottom }
                }
                Text {
                    visible: files.count === 0
                    x: 10
                    width: parent.width - 20
                    height: Theme.row
                    verticalAlignment: Text.AlignVCenter
                    text: Core.selected === "" ? "Select a profile to see its files." : "This profile lists no files."
                    font.family: Theme.sans
                    font.pixelSize: Theme.fBody
                    color: Theme.dim
                }
            }
            Rectangle {
                Layout.fillWidth: true
                height: Theme.row + 12
                color: Theme.header
                Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: Theme.rule }
                Kicker { id: ml; x: 10; height: parent.height; text: "MACHINE SECTIONS" }
                Text {
                    anchors { left: ml.right; leftMargin: 12; right: parent.right; rightMargin: 12 }
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                    text: (page.sel.machines || []).length > 0 ? page.sel.machines.map(function (m) { return "[machine." + m + "]" }).join("  ")
                                                              : "none: every key is derived from the machine it is applied on"
                    font.family: Theme.mono
                    font.pixelSize: Theme.fData
                    color: Theme.dim
                }
            }
        }
    }
}
