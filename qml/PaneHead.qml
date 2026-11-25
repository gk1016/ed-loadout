import QtQuick

// A docked pane's header strip: the title, an optional dim note, a hairline.
Rectangle {
    id: head
    property string title: ""
    property string note: ""
    property bool noteBad: false
    default property alias extras: slot.data

    height: Theme.headerH
    color: Theme.header
    Kicker {
        id: t
        x: 10
        height: parent.height
        text: head.title
        color: Theme.dim
    }
    Text {
        anchors { left: t.right; leftMargin: 12; right: slot.left; rightMargin: 8 }
        height: parent.height
        verticalAlignment: Text.AlignVCenter
        text: head.note
        elide: Text.ElideMiddle
        font.family: Theme.mono
        font.pixelSize: Theme.fMicro
        color: head.noteBad ? Theme.crit : Theme.faint
    }
    Row {
        id: slot
        anchors { right: parent.right; rightMargin: 6; verticalCenter: parent.verticalCenter }
        spacing: 6
    }
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.rule }
}
