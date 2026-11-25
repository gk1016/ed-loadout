import QtQuick

// The tool rail (FRAGO's Rail furniture at LOADOUT's 40 px): a glyph over a
// Barlow Condensed caption, the lit slot on a wash with a 2 px edge, SETTINGS
// apart at the bottom. Driven by Main.qml's tabDefs.
Rectangle {
    id: rail
    property var defs: []
    property string current: ""
    signal chosen(string id)

    readonly property int slotH: 44

    width: 40
    color: Theme.chrome
    Rectangle { anchors { top: parent.top; bottom: parent.bottom; right: parent.right } width: 1; color: Theme.rule }

    component Slot: Item {
        id: slot
        required property var d
        readonly property bool lit: d.id === rail.current
        width: rail.width - 1
        height: rail.slotH

        Rectangle {
            anchors.fill: parent
            color: slot.lit ? Theme.wash(0.14) : (hov.hovered ? Theme.hover(0.05) : "transparent")
        }
        Rectangle { visible: slot.lit; width: 2; height: parent.height; color: Theme.sand }
        Column {
            anchors.centerIn: parent
            spacing: 2
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: slot.d.g
                color: slot.lit ? Theme.sandHi : Theme.dim
                // Named, so the gear is the monochrome symbol and not the
                // colour emoji the fallback chain would pick.
                font.family: "Segoe UI Symbol"
                font.pixelSize: 15
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: slot.d.label
                color: slot.lit ? Theme.sandHi : Theme.dim
                font.family: Theme.cond
                font.pixelSize: 9
                font.weight: Font.Medium
                font.letterSpacing: 0.3
            }
        }
        HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
        // TapHandler, not MouseArea.onClicked: a MouseArea drops `clicked`
        // when it loses hover between press and release.
        TapHandler { onTapped: rail.chosen(slot.d.id) }
    }

    Column {
        y: 0
        Repeater {
            model: rail.defs.filter(function (d) { return d.id !== "settings" })
            delegate: Slot { required property var modelData; d: modelData }
        }
    }
    // SETTINGS sits at the bottom, apart from the workspaces.
    Repeater {
        model: rail.defs.filter(function (d) { return d.id === "settings" })
        delegate: Slot {
            required property var modelData
            d: modelData
            anchors.bottom: parent.bottom
        }
    }
}
