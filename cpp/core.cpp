#include "core.h"

#include "loadout-core/src/bridge.rs.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDebug>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPointer>
#include <QSettings>
#include <QStandardPaths>
#include <QThreadPool>

namespace {
// A bridge result: UTF-8 JSON object -> QVariantMap.
QVariantMap parsed(const rust::String &s)
{
    const QJsonDocument doc = QJsonDocument::fromJson(QByteArray(s.data(), qsizetype(s.size())));
    if (!doc.isObject())
        return { { QStringLiteral("ok"), false }, { QStringLiteral("error"), QStringLiteral("the core returned no JSON object") } };
    return doc.object().toVariantMap();
}

// The bytes stay alive in the caller's QByteArray for the length of the call.
rust::Str rs(const QByteArray &utf8)
{
    return rust::Str(utf8.constData(), size_t(utf8.size()));
}

QString native(const QString &path)
{
    return QDir::toNativeSeparators(path);
}

QString localAppData()
{
    const QString env = qEnvironmentVariable("LOCALAPPDATA");
    return env.isEmpty() ? QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation) : QDir::fromNativeSeparators(env);
}

QSettings settings()
{
    return QSettings(QSettings::IniFormat, QSettings::UserScope, QStringLiteral("Boring Tech Stuff"), QStringLiteral("LOADOUT"));
}

bool copyTree(const QString &from, const QString &to, QString *error)
{
    if (!QDir().mkpath(to)) {
        *error = to + QStringLiteral(": cannot create");
        return false;
    }
    const QDir src(from);
    for (const QFileInfo &e : src.entryInfoList(QDir::Files | QDir::Dirs | QDir::NoDotAndDotDot | QDir::Hidden)) {
        const QString dst = to + QLatin1Char('/') + e.fileName();
        if (e.isDir()) {
            if (!copyTree(e.absoluteFilePath(), dst, error))
                return false;
        } else if (!QFile::copy(e.absoluteFilePath(), dst)) {
            *error = dst + QStringLiteral(": cannot copy");
            return false;
        }
    }
    return true;
}
} // namespace

Core::Core(const QString &harnessOptions, const QString &harnessProfiles, QObject *parent)
    : QObject(parent)
{
    m_harness = !harnessOptions.isEmpty() || !harnessProfiles.isEmpty();
    if (m_harness) {
        m_optionsOverride = QDir::fromNativeSeparators(harnessOptions);
        m_profilesDir = QDir::fromNativeSeparators(harnessProfiles);
    } else {
        QSettings s = settings();
        m_optionsOverride = s.value(QStringLiteral("paths/options")).toString();
        m_profilesDir = s.value(QStringLiteral("paths/profiles")).toString();
        m_selected = s.value(QStringLiteral("ui/selected")).toString();
        if (s.value(QStringLiteral("ui/theme")).toString() == QStringLiteral("daylight"))
            m_theme = QStringLiteral("daylight");
    }

    const QVariantMap d = parsed(loadout::detect());
    m_detected = d;
    refreshProfiles();
    refreshLastBackup();

    m_gameTimer.setInterval(3000);
    connect(&m_gameTimer, &QTimer::timeout, this, &Core::pollGame);
    m_gameTimer.start();
    // The first answer before the first frame: APPLY must not flash enabled.
    m_gameRunning = loadout::game_running();
}

QString Core::defaultOptionsDir() const
{
    return native(localAppData() + QStringLiteral("/Frontier Developments/Elite Dangerous/Options"));
}

QString Core::optionsDir() const
{
    return m_optionsOverride.isEmpty() ? defaultOptionsDir() : native(m_optionsOverride);
}

bool Core::optionsOk() const
{
    return QDir(optionsDir()).exists();
}

QString Core::optionsOverride() const
{
    return native(m_optionsOverride);
}

void Core::setOptionsOverride(const QString &dir)
{
    const QString d = QDir::fromNativeSeparators(dir.trimmed());
    if (d == m_optionsOverride)
        return;
    m_optionsOverride = d;
    save(QStringLiteral("paths/options"), d);
    emit optionsChanged();
    refreshLastBackup();
    replan();
}

QString Core::defaultProfilesDir() const
{
    return native(localAppData() + QStringLiteral("/LOADOUT/profiles"));
}

QString Core::profilesDir() const
{
    return m_profilesDir.isEmpty() ? defaultProfilesDir() : native(m_profilesDir);
}

void Core::setProfilesDir(const QString &dir)
{
    const QString d = QDir::fromNativeSeparators(dir.trimmed());
    if (d == m_profilesDir)
        return;
    m_profilesDir = d;
    save(QStringLiteral("paths/profiles"), d);
    emit profilesDirChanged();
    refreshProfiles();
}

void Core::setTheme(const QString &name)
{
    const QString t = name == QStringLiteral("daylight") ? name : QStringLiteral("deepfield");
    if (t == m_theme)
        return;
    m_theme = t;
    save(QStringLiteral("ui/theme"), t);
    emit themeChanged();
}

void Core::save(const QString &key, const QString &value)
{
    if (m_harness)
        return;
    QSettings s = settings();
    if (value.isEmpty())
        s.remove(key);
    else
        s.setValue(key, value);
}

QVariantMap Core::selectedProfile() const
{
    for (const QVariant &p : m_profiles) {
        const QVariantMap m = p.toMap();
        if (m.value(QStringLiteral("path")).toString() == m_selected)
            return m;
    }
    return {};
}

void Core::say(const QString &line)
{
    m_log.append(line);
    // The log file carries the same lines (ship.ps1 reads them back).
    qInfo().noquote() << "core:" << line;
}

void Core::sayAll(const QVariantMap &result)
{
    for (const QVariant &l : result.value(QStringLiteral("log")).toList())
        say(l.toString());
    if (!result.value(QStringLiteral("ok")).toBool())
        say(QStringLiteral("error: ") + result.value(QStringLiteral("error")).toString());
    // Bounded: the page is a view of recent work, not an archive.
    while (m_log.size() > 2000)
        m_log.removeFirst();
    emit logChanged();
}

void Core::setNotice(const QString &text, bool bad)
{
    m_notice = text;
    m_noticeBad = bad;
    emit noticeChanged();
}

void Core::clearLog()
{
    m_log.clear();
    emit logChanged();
}

void Core::detectAgain()
{
    say(QStringLiteral("> detect  %1").arg(QDateTime::currentDateTime().toString(Qt::ISODate)));
    m_detected = parsed(loadout::detect());
    const QVariantMap f = m_detected.value(QStringLiteral("facts")).toMap();
    if (m_detected.value(QStringLiteral("ok")).toBool()) {
        say(QStringLiteral("hostname: %1").arg(f.value(QStringLiteral("hostname")).toString()));
        say(QStringLiteral("panel: %1x%2 @ %3 Hz").arg(f.value(QStringLiteral("screen_w")).toString(), f.value(QStringLiteral("screen_h")).toString(), f.value(QStringLiteral("refresh_hz")).toString()));
        say(QStringLiteral("gpu: %1 (%2 MB)").arg(f.value(QStringLiteral("gpu_name")).toString(), f.value(QStringLiteral("gpu_vram_mb")).toString()));
        const QStringList devices = f.value(QStringLiteral("devices")).toStringList();
        say(QStringLiteral("devices: %1").arg(devices.isEmpty() ? QStringLiteral("(none)") : devices.join(QStringLiteral(", "))));
        say(QStringLiteral("tier: %1").arg(m_detected.value(QStringLiteral("tier")).toString()));
        for (const QVariant &r : m_detected.value(QStringLiteral("derived")).toList()) {
            const QVariantMap m = r.toMap();
            say(QStringLiteral("derived %1 = %2  (%3)").arg(m.value(QStringLiteral("key")).toString(), m.value(QStringLiteral("value")).toString(), m.value(QStringLiteral("reason")).toString()));
        }
    }
    sayAll(m_detected);
    emit detectedChanged();
    pollGame();
    replan();
    setNotice(m_detected.value(QStringLiteral("ok")).toBool() ? QStringLiteral("Detected again.") : m_detected.value(QStringLiteral("error")).toString(),
              !m_detected.value(QStringLiteral("ok")).toBool());
}

void Core::refreshProfiles()
{
    const QByteArray dir = profilesDir().toUtf8();
    const QVariantMap r = parsed(loadout::list_profiles(rs(dir)));
    m_profiles = r.value(QStringLiteral("profiles")).toList();
    if (!r.value(QStringLiteral("ok")).toBool())
        sayAll(r);
    emit profilesChanged();
    // A selection whose folder is gone is no selection.
    if (!m_selected.isEmpty() && selectedProfile().isEmpty()) {
        m_selected.clear();
        save(QStringLiteral("ui/selected"), m_selected);
    }
    emit selectedChanged();
    replan();
}

void Core::select(const QString &profilePath)
{
    if (profilePath == m_selected)
        return;
    m_selected = profilePath;
    save(QStringLiteral("ui/selected"), m_selected);
    emit selectedChanged();
    replan();
    const QVariantMap p = selectedProfile();
    qInfo().noquote() << QStringLiteral("select: %1, %2 effective rows, %3 files to write")
                             .arg(p.value(QStringLiteral("name")).toString())
                             .arg(m_plan.value(QStringLiteral("rows")).toList().size())
                             .arg(m_plan.value(QStringLiteral("files")).toList().size());
}

void Core::replan()
{
    QVariantMap plan;
    if (!m_selected.isEmpty()) {
        const QByteArray profile = m_selected.toUtf8();
        const QByteArray options = optionsDir().toUtf8();
        plan = parsed(loadout::plan_apply(rs(profile), rs(options), rs(QByteArray())));
    }
    m_plan = plan;
    emit planChanged();
}

void Core::refreshLastBackup()
{
    const QByteArray options = optionsDir().toUtf8();
    const QString stamp = parsed(loadout::last_backup(rs(options))).value(QStringLiteral("stamp")).toString();
    if (stamp != m_lastBackup) {
        m_lastBackup = stamp;
        emit lastBackupChanged();
    }
}

void Core::pollGame()
{
    // tasklist takes a few hundred ms: off the GUI thread, one at a time.
    if (m_polling.exchange(true))
        return;
    QPointer<Core> self(this);
    QThreadPool::globalInstance()->start([self]() {
        const bool running = loadout::game_running();
        QMetaObject::invokeMethod(qApp, [self, running]() {
            if (!self)
                return;
            self->m_polling = false;
            if (running != self->m_gameRunning) {
                self->m_gameRunning = running;
                emit self->gameRunningChanged();
            }
        }, Qt::QueuedConnection);
    });
}

bool Core::captureThisMachine()
{
    if (!optionsOk()) {
        setNotice(QStringLiteral("Options dir not found: %1").arg(optionsDir()), true);
        return false;
    }
    QString machine = m_detected.value(QStringLiteral("facts")).toMap().value(QStringLiteral("hostname")).toString();
    if (machine.isEmpty())
        machine = qEnvironmentVariable("COMPUTERNAME", QStringLiteral("this-machine"));
    const QString root = QDir::fromNativeSeparators(profilesDir());
    QDir().mkpath(root);
    // <hostname>, then <hostname>-<stamp>: a capture never overwrites one.
    QString out = root + QLatin1Char('/') + machine;
    if (QFileInfo::exists(out))
        out += QLatin1Char('-') + QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-HHmmss"));
    out = native(out);
    say(QStringLiteral("> capture --options \"%1\" --out \"%2\" --machine %3").arg(optionsDir(), out, machine));
    const QByteArray o = optionsDir().toUtf8(), p = out.toUtf8(), m = machine.toUtf8();
    const QVariantMap r = parsed(loadout::capture(rs(o), rs(p), rs(m)));
    sayAll(r);
    const bool ok = r.value(QStringLiteral("ok")).toBool();
    refreshProfiles();
    if (ok && m_selected.isEmpty())
        select(out);
    setNotice(ok ? QStringLiteral("Captured %1.").arg(QFileInfo(out).fileName()) : r.value(QStringLiteral("error")).toString(), !ok);
    return ok;
}

void Core::importFolder()
{
    const QString dir = QFileDialog::getExistingDirectory(nullptr, QStringLiteral("Import a LOADOUT profile folder"));
    if (!dir.isEmpty())
        importFrom(dir);
}

bool Core::importFrom(const QString &dir)
{
    const QString from = QDir::fromNativeSeparators(dir);
    const QString name = QFileInfo(from).fileName();
    const QString to = QDir::fromNativeSeparators(profilesDir()) + QLatin1Char('/') + name;
    say(QStringLiteral("> import \"%1\"").arg(native(from)));
    QString error;
    if (!QFileInfo::exists(from + QStringLiteral("/profile.toml")) || !QFileInfo(from + QStringLiteral("/files")).isDir())
        error = QStringLiteral("%1: not a profile (needs profile.toml and files/)").arg(native(from));
    else if (QFileInfo::exists(to))
        error = QStringLiteral("%1: already exists; refusing to overwrite").arg(native(to));
    else if (copyTree(from, to, &error))
        say(QStringLiteral("imported %1").arg(native(to)));
    if (!error.isEmpty())
        say(QStringLiteral("error: ") + error);
    emit logChanged();
    refreshProfiles();
    if (error.isEmpty() && m_selected.isEmpty())
        select(native(to));
    setNotice(error.isEmpty() ? QStringLiteral("Imported %1.").arg(name) : error, !error.isEmpty());
    return error.isEmpty();
}

void Core::browseProfilesDir()
{
    const QString dir = QFileDialog::getExistingDirectory(nullptr, QStringLiteral("Profiles folder"), profilesDir());
    if (!dir.isEmpty())
        setProfilesDir(dir);
}

void Core::browseOptionsDir()
{
    const QString dir = QFileDialog::getExistingDirectory(nullptr, QStringLiteral("Elite Dangerous Options dir"), optionsDir());
    if (!dir.isEmpty())
        setOptionsOverride(dir);
}

bool Core::apply()
{
    if (m_selected.isEmpty())
        return false;
    say(QStringLiteral("> apply \"%1\" --options \"%2\"").arg(m_selected, optionsDir()));
    const QByteArray profile = m_selected.toUtf8(), options = optionsDir().toUtf8();
    const QVariantMap r = parsed(loadout::apply(rs(profile), rs(options), rs(QByteArray())));
    sayAll(r);
    const bool ok = r.value(QStringLiteral("ok")).toBool();
    refreshLastBackup();
    replan();
    pollGame();
    setNotice(ok ? QStringLiteral("Applied %1. Backup %2.").arg(selectedProfile().value(QStringLiteral("name")).toString(), r.value(QStringLiteral("stamp")).toString())
                 : r.value(QStringLiteral("error")).toString(),
              !ok);
    return ok;
}

bool Core::restoreLast()
{
    say(QStringLiteral("> restore --options \"%1\"  (last backup %2)").arg(optionsDir(), m_lastBackup.isEmpty() ? QStringLiteral("none") : m_lastBackup));
    const QByteArray options = optionsDir().toUtf8();
    const QVariantMap r = parsed(loadout::restore(rs(options), rs(QByteArray())));
    sayAll(r);
    const bool ok = r.value(QStringLiteral("ok")).toBool();
    refreshLastBackup();
    replan();
    pollGame();
    setNotice(ok ? QStringLiteral("Restored backup %1.").arg(r.value(QStringLiteral("stamp")).toString()) : r.value(QStringLiteral("error")).toString(), !ok);
    return ok;
}
