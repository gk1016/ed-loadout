import QtQuick

// The LOADOUT mark: a bracketed hexagon with a horizontal rule. The same
// 64-unit geometry as icons/loadout.svg (tools/make_icon.py), drawn in the
// live theme's accent so it follows Deep Field / Daylight.
Canvas {
    id: mark
    property real size: 18
    property color ink: Theme.sand

    width: size
    height: size
    antialiasing: true
    onInkChanged: requestPaint()
    onSizeChanged: requestPaint()

    onPaint: {
        const ctx = getContext("2d")
        ctx.reset()
        ctx.clearRect(0, 0, width, height)
        ctx.scale(width / 64, height / 64)
        ctx.strokeStyle = mark.ink
        ctx.lineWidth = 5
        ctx.lineJoin = "miter"
        // brackets
        ctx.beginPath(); ctx.moveTo(15, 9); ctx.lineTo(6, 9); ctx.lineTo(6, 55); ctx.lineTo(15, 55); ctx.stroke()
        ctx.beginPath(); ctx.moveTo(49, 9); ctx.lineTo(58, 9); ctx.lineTo(58, 55); ctx.lineTo(49, 55); ctx.stroke()
        // hexagon, point up, radius 16 about (32, 32)
        ctx.beginPath()
        for (let i = 0; i < 6; i++) {
            const a = (-90 + 60 * i) * Math.PI / 180
            const x = 32 + 16 * Math.cos(a), y = 32 + 16 * Math.sin(a)
            if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y)
        }
        ctx.closePath(); ctx.stroke()
        // the rule
        const half = 16 * Math.cos(Math.PI / 6)
        ctx.beginPath(); ctx.moveTo(32 - half, 32); ctx.lineTo(32 + half, 32); ctx.stroke()
    }
}
