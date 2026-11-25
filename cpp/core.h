// The QObject between QML and the Rust core (rust/src/bridge.rs). Every
// bridge call returns one JSON object; Core turns it into properties the
// pages bind to and appends the core's own lines to the LOG page.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

#include <atomic>

class Core : public QObject
{
    Q_OBJECT
    // {ok, error, facts: {hostname, screen_w, screen_h, refresh_hz, gpu_name,
    //  gpu_vram_mb, devices}, tier, derived: [{key, value, reason}]}
    Q_PROPERTY(QVariantMap detected READ detected NOTIFY detectedChanged)
    // [{name, path, source_machine, captured_at, files: [{rel, sha256}], machines, error}]
    Q_PROPERTY(QVariantList profiles READ profiles NOTIFY profilesChanged)
    // Path of the selected profile, "" for none; and its entry from `profiles`.
    Q_PROPERTY(QString selected READ selected NOTIFY selectedChanged)
    Q_PROPERTY(QVariantMap selectedProfile READ selectedProfile NOTIFY selectedChanged)
    // {ok, error, rows, files, edits, unconnected, facts, backup_root}; empty with no profile.
    Q_PROPERTY(QVariantMap plan READ plan NOTIFY planChanged)
    Q_PROPERTY(bool gameRunning READ gameRunning NOTIFY gameRunningChanged)
    Q_PROPERTY(QString optionsDir READ optionsDir NOTIFY optionsChanged)
    Q_PROPERTY(bool optionsOk READ optionsOk NOTIFY optionsChanged)
    Q_PROPERTY(QString optionsOverride READ optionsOverride WRITE setOptionsOverride NOTIFY optionsChanged)
    Q_PROPERTY(QString defaultOptionsDir READ defaultOptionsDir CONSTANT)
    Q_PROPERTY(QString profilesDir READ profilesDir WRITE setProfilesDir NOTIFY profilesDirChanged)
    Q_PROPERTY(QString defaultProfilesDir READ defaultProfilesDir CONSTANT)
    // deepfield | daylight. Saved here, not by QML: a harness run saves nothing.
    Q_PROPERTY(QString theme READ theme WRITE setTheme NOTIFY themeChanged)
    // Stamp of the newest apply backup under the Options dir, "" for none.
    Q_PROPERTY(QString lastBackup READ lastBackup NOTIFY lastBackupChanged)
    Q_PROPERTY(QStringList log READ log NOTIFY logChanged)
    // The outcome of the last action, for the action bar; bad = it failed.
    Q_PROPERTY(QString notice READ notice NOTIFY noticeChanged)
    Q_PROPERTY(bool noticeBad READ noticeBad NOTIFY noticeChanged)

public:
    // harnessOptions / harnessProfiles: used instead of the saved settings
    // and never saved (ship.ps1's scripted run, shots.ps1).
    Core(const QString &harnessOptions, const QString &harnessProfiles, QObject *parent = nullptr);

    QVariantMap detected() const { return m_detected; }
    QVariantList profiles() const { return m_profiles; }
    QString selected() const { return m_selected; }
    QVariantMap selectedProfile() const;
    QVariantMap plan() const { return m_plan; }
    bool gameRunning() const { return m_gameRunning; }
    QString optionsDir() const;
    bool optionsOk() const;
    QString optionsOverride() const;
    void setOptionsOverride(const QString &dir);
    QString defaultOptionsDir() const;
    QString profilesDir() const;
    void setProfilesDir(const QString &dir);
    QString defaultProfilesDir() const;
    QString theme() const { return m_theme; }
    void setTheme(const QString &name);
    QString lastBackup() const { return m_lastBackup; }
    QStringList log() const { return m_log; }
    QString notice() const { return m_notice; }
    bool noticeBad() const { return m_noticeBad; }

    Q_INVOKABLE void detectAgain();
    Q_INVOKABLE void refreshProfiles();
    Q_INVOKABLE void select(const QString &profilePath);
    Q_INVOKABLE bool captureThisMachine();
    // Opens a folder dialog, then importFrom().
    Q_INVOKABLE void importFolder();
    Q_INVOKABLE bool importFrom(const QString &dir);
    Q_INVOKABLE void browseProfilesDir();
    Q_INVOKABLE void browseOptionsDir();
    Q_INVOKABLE bool apply();
    Q_INVOKABLE bool restoreLast();
    Q_INVOKABLE void clearLog();

signals:
    void detectedChanged();
    void profilesChanged();
    void selectedChanged();
    void planChanged();
    void gameRunningChanged();
    void optionsChanged();
    void profilesDirChanged();
    void themeChanged();
    void lastBackupChanged();
    void logChanged();
    void noticeChanged();

private:
    void replan();
    void refreshLastBackup();
    void pollGame();
    void say(const QString &line);
    void sayAll(const QVariantMap &result);
    void setNotice(const QString &text, bool bad);
    void save(const QString &key, const QString &value);

    bool m_harness = false;
    QString m_optionsOverride;
    QString m_profilesDir;
    QString m_theme = QStringLiteral("deepfield");
    QVariantMap m_detected;
    QVariantList m_profiles;
    QString m_selected;
    QVariantMap m_plan;
    bool m_gameRunning = false;
    QString m_lastBackup;
    QStringList m_log;
    QString m_notice;
    bool m_noticeBad = false;
    QTimer m_gameTimer;
    std::atomic<bool> m_polling{ false };
};
