import QtQuick

// The one button (FRAGO's Btn, in Barlow Condensed: LOADOUT ships the three
// families only). Flat, hairline, sand when it is the primary action.
Rectangle {
    id: b
    property string label: ""
    property bool primary: false
    property bool danger: false
    property bool live: true
    signal clicked()

    implicitWidth: Math.max(84, t.implicitWidth + 28)
    implicitHeight: Theme.row
    radius: Theme.radius
    // Disabled is its own flat state, not a faded copy of the live one.
    color: !live ? Theme.pane
         : primary ? (ma.containsMouse ? Theme.sandHi : Theme.sand)
                   : (ma.containsMouse ? Theme.hover(0.07) : "transparent")
    border.width: primary && live ? 0 : 1
    border.color: !live ? Theme.rule : danger ? Theme.crit : (ma.containsMouse ? Theme.ruleHi : Theme.rule)

    Text {
        id: t
        anchors.centerIn: parent
        text: b.label
        font.family: Theme.cond
        font.pixelSize: Theme.fLabel + 2
        font.weight: Font.DemiBold
        font.letterSpacing: 1.2
        color: !b.live ? (Theme.light ? Theme.dim : Theme.faint)
             : b.primary ? Theme.onAccent : (b.danger ? Theme.crit : Theme.ink)
    }
    MouseArea {
        id: ma
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: b.live ? Qt.PointingHandCursor : Qt.ArrowCursor
        // Not onClicked: inside a ListView the press drops this area's hover
        // state, and MouseArea suppresses clicked without it.
        onReleased: (m) => { if (b.live && m.x >= 0 && m.y >= 0 && m.x < width && m.y < height) b.clicked() }
    }
}
