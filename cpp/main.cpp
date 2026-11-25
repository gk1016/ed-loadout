// LOADOUT - the desktop app over loadout-core: what this machine is, what
// will be applied, APPLY, RESTORE. Plan: claude/plans/2026-11-25-loadout-v0.3-desktop.md.

#include "core.h"

#include <QApplication>
#include <QCommandLineParser>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFontDatabase>
#include <QIcon>
#include <QMutex>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTextStream>
#include <QTimer>

#ifndef LOADOUT_VERSION
#define LOADOUT_VERSION "0.0.0"
#endif
#ifndef LOADOUT_BUILD_DATE
#define LOADOUT_BUILD_DATE "unknown"
#endif
#ifndef LOADOUT_GIT_SHA
#define LOADOUT_GIT_SHA "nogit"
#endif

namespace {
QFile *g_log = nullptr;
QMutex g_logMutex;

// loadout.log beside the exe: no console window on a GUI build, and Qt's own
// logging goes to OutputDebugString on Windows.
void logHandler(QtMsgType type, const QMessageLogContext &, const QString &msg)
{
    QMutexLocker lock(&g_logMutex);
    if (!g_log)
        return;
    const char *lvl = type == QtWarningMsg ? "WARN" : type == QtCriticalMsg ? "CRIT" : type == QtFatalMsg ? "FATAL" : "info";
    QTextStream(g_log) << QDateTime::currentDateTimeUtc().toString(Qt::ISODate) << ' ' << lvl << ' ' << msg << '\n';
    g_log->flush();
}
} // namespace

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("LOADOUT"));
    app.setOrganizationName(QStringLiteral("Boring Tech Stuff"));
    app.setApplicationVersion(QStringLiteral(LOADOUT_VERSION));

    QCommandLineParser parser;
    parser.addHelpOption();
    QCommandLineOption shotOpt(QStringLiteral("shot"), QStringLiteral("Save a window capture to <png> (harness)."), QStringLiteral("png"));
    QCommandLineOption tabOpt(QStringLiteral("tab"), QStringLiteral("Open on <tab>: home, profiles, log, settings (harness)."), QStringLiteral("tab"));
    QCommandLineOption themeOpt(QStringLiteral("theme"), QStringLiteral("Open in <theme> without saving it (harness)."), QStringLiteral("theme"));
    QCommandLineOption quitOpt(QStringLiteral("quit-after"), QStringLiteral("Quit after <ms> (harness)."), QStringLiteral("ms"));
    QCommandLineOption optionsOpt(QStringLiteral("options"), QStringLiteral("Use <dir> as the Options dir, without saving it (harness)."), QStringLiteral("dir"));
    QCommandLineOption profilesOpt(QStringLiteral("profiles"), QStringLiteral("Use <dir> as the profiles folder, without saving it (harness)."), QStringLiteral("dir"));
    QCommandLineOption selectOpt(QStringLiteral("select"), QStringLiteral("Select the profile named <name>, without saving it (harness)."), QStringLiteral("name"));
    QCommandLineOption confirmOpt(QStringLiteral("confirm"), QStringLiteral("Open the APPLY confirm (harness)."));
    QCommandLineOption scriptOpt(QStringLiteral("script"), QStringLiteral("Run <steps> through the same controls a click reaches: capture,apply,restore,log (harness)."), QStringLiteral("steps"));
    parser.addOptions({ shotOpt, tabOpt, themeOpt, quitOpt, optionsOpt, profilesOpt, selectOpt, confirmOpt, scriptOpt });
    parser.process(app);

    const QString appDir = QCoreApplication::applicationDirPath();
    QFile logFile(appDir + QStringLiteral("/loadout.log"));
    if (logFile.size() > 2 * 1024 * 1024)
        logFile.remove();
    if (logFile.open(QIODevice::Append | QIODevice::Text)) {
        g_log = &logFile;
        qInstallMessageHandler(logHandler);
    }
    qInfo().noquote() << QStringLiteral("LOADOUT %1 built %2 @ %3 in %4")
                             .arg(QStringLiteral(LOADOUT_VERSION), QStringLiteral(LOADOUT_BUILD_DATE),
                                  QStringLiteral(LOADOUT_GIT_SHA), appDir);

    app.setWindowIcon(QIcon(appDir + QStringLiteral("/icons/loadout.ico")));
    const QDir fontDir(appDir + QStringLiteral("/fonts"));
    for (const QString &f : fontDir.entryList({ QStringLiteral("*.ttf") }, QDir::Files))
        QFontDatabase::addApplicationFont(fontDir.filePath(f));
    QQuickStyle::setStyle(QStringLiteral("Basic"));

    // The context object BEFORE the engine: destruction runs in reverse, and
    // an object that dies first leaves QML bindings pointing at freed memory.
    Core core(parser.value(optionsOpt), parser.value(profilesOpt));
    const QVariantMap buildInfo{
        { QStringLiteral("version"), QStringLiteral(LOADOUT_VERSION) },
        { QStringLiteral("built"), QStringLiteral(LOADOUT_BUILD_DATE) },
        { QStringLiteral("sha"), QStringLiteral(LOADOUT_GIT_SHA) },
    };

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty(QStringLiteral("Core"), &core);
    engine.rootContext()->setContextProperty(QStringLiteral("BuildInfo"), buildInfo);
    engine.rootContext()->setContextProperty(QStringLiteral("StartTab"), parser.value(tabOpt));
    engine.rootContext()->setContextProperty(QStringLiteral("StartTheme"), parser.value(themeOpt));
    engine.rootContext()->setContextProperty(QStringLiteral("StartSelect"), parser.value(selectOpt));
    engine.rootContext()->setContextProperty(QStringLiteral("StartConfirm"), parser.isSet(confirmOpt));
    engine.rootContext()->setContextProperty(QStringLiteral("StartScript"),
                                             parser.value(scriptOpt).split(QLatin1Char(','), Qt::SkipEmptyParts));
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreationFailed, &app,
                     []() { QCoreApplication::exit(1); }, Qt::QueuedConnection);
    engine.load(QUrl::fromLocalFile(appDir + QStringLiteral("/qml/Main.qml")));
    if (engine.rootObjects().isEmpty()) {
        qCritical() << "Main.qml did not load";
        return 1;
    }

    if (parser.isSet(shotOpt)) {
        const QString path = parser.value(shotOpt);
        auto *win = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
        const int delay = parser.isSet(quitOpt) ? qMax(500, parser.value(quitOpt).toInt() - 800) : 5000;
        QTimer::singleShot(delay, &app, [win, path]() {
            if (win && win->grabWindow().save(path))
                qInfo().noquote() << "shot:" << path;
            else
                qWarning().noquote() << "shot failed:" << path;
        });
    }
    if (parser.isSet(quitOpt))
        QTimer::singleShot(parser.value(quitOpt).toInt(), &app, &QCoreApplication::quit);

    const int rc = app.exec();
    qInstallMessageHandler(nullptr);
    g_log = nullptr;
    return rc;
}
