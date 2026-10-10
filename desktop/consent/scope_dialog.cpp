#include "scope_dialog.hpp"
#include <QDialogButtonBox>
#include <QJsonArray>
#include <QFile>
#include <QJsonDocument>
#include <QLabel>
#include <QPushButton>
#include <QRegularExpression>
#include <QScrollArea>
#include <QSet>
#include <QTimer>
#include <QVBoxLayout>
#include <limits>
#include <time.h>
#include <unistd.h>

quint64 boottimeMs() {
    timespec t{};
    if (clock_gettime(CLOCK_BOOTTIME, &t) != 0) return 0;
    return quint64(t.tv_sec) * 1000 + quint64(t.tv_nsec) / 1000000;
}
static bool keys(const QJsonObject &o, const QStringList &expected) {
    auto actual = o.keys(); auto sorted = expected; sorted.sort();
    return actual == sorted;
}
static bool text(const QJsonValue &v, int limit) {
    if (!v.isString() || v.toString().isEmpty() || v.toString().toUtf8().size() > limit) return false;
    for (auto c : v.toString()) {
        if (c.category() == QChar::Other_Control && c != '\n' && c != '\t') return false;
        // Permission boundaries must not be visually reordered by supplied
        // strings. Native names are observations, not trusted markup.
        auto n=c.unicode();
        if (n==0x200e || n==0x200f || (n>=0x202a && n<=0x202e) || (n>=0x2066 && n<=0x2069)) return false;
    }
    return true;
}
static bool integer(const QJsonValue &v, quint64 &out) {
    if (!v.isDouble()) return false;
    double n = v.toDouble();
    if (n < 0 || n > 9007199254740991.0 || n != double(quint64(n))) return false;
    out = quint64(n); return true;
}
std::optional<ScopePreview> ScopePreview::parse(const QByteArray &canonical) {
    if (canonical.isEmpty() || canonical.size() > 65536) return {};
    QJsonParseError error;
    auto parsed = QJsonDocument::fromJson(canonical, &error);
    // Requiring the canonical wire form also rejects duplicate keys, trailing
    // input and floating point spellings. No duplicate-collapsing parse wins.
    if (error.error != QJsonParseError::NoError || !parsed.isObject() || parsed.toJson(QJsonDocument::Compact) != canonical) return {};
    auto o = parsed.object();
    const bool termination = o["kind"] == "process_termination";
    const bool powerProfile = o["kind"] == "power_profile_change";
    const QStringList fields = termination
        ? QStringList{"schema_version","kind","digest","uid","session_id","target","profile","mode","goal","process_id","preview","closure","actions","issued_ms","expires_ms","evidence"}
        : powerProfile ? QStringList{"schema_version","kind","digest","uid","session_id","target","profile","mode","goal","preview","actions","issued_ms","expires_ms","evidence"}
        : QStringList{"schema_version","kind","digest","uid","session_id","target","profile","mode","goal","apps","actions","issued_ms","expires_ms","evidence"};
    if (!keys(o, fields) || o["schema_version"] != 1 || (!termination && !powerProfile && o["kind"] != "read_scope")) return {};
    quint64 uid, issued, expires;
    auto now = boottimeMs();
    if (!integer(o["uid"], uid) || uid != geteuid() || uid == 0 || !integer(o["issued_ms"], issued) || !integer(o["expires_ms"], expires) || !now || issued > now || expires <= now || expires <= issued || expires-issued > 300000) return {};
    static const QRegularExpression hash("^[0-9a-f]{64}$");
    static const QRegularExpression session("^[A-Za-z0-9_-]{1,128}$");
    if (!text(o["digest"],64) || !hash.match(o["digest"].toString()).hasMatch() || !text(o["session_id"],128) || !session.match(o["session_id"].toString()).hasMatch() || !text(o["target"],256) || !text(o["profile"],256) || !text(o["goal"],4096) || ((termination || powerProfile) ? o["mode"] != "act" : (o["mode"] != "ask" && o["mode"] != "diagnose"))) return {};
    if (!o["evidence"].isArray() || o["evidence"].toArray().size() > 16) return {};
    for (auto e : o["evidence"].toArray()) if (!text(e,128)) return {};
    if (termination) {
        static const QRegularExpression uuid("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$");
        static const QRegularExpression closure("^/nix/store/[0123456789abcdfghijklmnpqrsvwxyz]{32}-[A-Za-z0-9+._?=-]+$");
        if (!text(o["process_id"],36) || !uuid.match(o["process_id"].toString()).hasMatch()
            || o["process_id"] == "00000000-0000-0000-0000-000000000000" || !text(o["closure"],128)
            || !closure.match(o["closure"].toString()).hasMatch() || !o["actions"].isArray()
            || o["actions"].toArray() != QJsonArray{"process.terminate"} || !o["preview"].isObject()) return {};
        auto p=o["preview"].toObject(); quint64 timeout;
        if (!keys(p,{"identity","signal","verification_timeout_ms","reversible","automatic_escalation"})
            || p["signal"] != "SIGTERM" || p["reversible"] != false || p["automatic_escalation"] != false
            || !integer(p["verification_timeout_ms"],timeout) || !timeout || timeout>30000 || !p["identity"].isObject()) return {};
        auto i=p["identity"].toObject(); quint64 pid,start,processUid;
        if (!keys(i,{"pid","uid","start_time_ticks","boot_id","executable_identity"})
            || !integer(i["pid"],pid) || pid<=1 || pid>4294967295ULL || pid==quint64(getpid())
            || !integer(i["uid"],processUid) || processUid!=uid || !integer(i["start_time_ticks"],start) || !start
            || !text(i["boot_id"],36) || !uuid.match(i["boot_id"].toString()).hasMatch()
            || !text(i["executable_identity"],256)) return {};
        QFile boot("/proc/sys/kernel/random/boot_id");
        if (!boot.open(QIODevice::ReadOnly) || QString::fromLatin1(boot.read(64)).trimmed()!=i["boot_id"].toString()) return {};
    } else if (powerProfile) {
        if (!o["actions"].isArray() || o["actions"].toArray()!=QJsonArray{"power.profile_set"} || !o["preview"].isObject()) return {};
        auto p=o["preview"].toObject();
        if (!keys(p,{"prior","requested","available_profiles","reversible"}) || p["reversible"]!=true
            || !text(p["prior"],32) || !text(p["requested"],32) || !p["available_profiles"].isArray()
            || p["available_profiles"].toArray().isEmpty() || p["available_profiles"].toArray().size()>3) return {};
        QSet<QString> profiles;
        for (auto value:p["available_profiles"].toArray()) {
            if (!value.isString() || (value!="power-saver" && value!="balanced" && value!="performance")
                || profiles.contains(value.toString())) return {};
            profiles.insert(value.toString());
        }
        if (!profiles.contains(p["prior"].toString()) || !profiles.contains(p["requested"].toString())) return {};
    } else {
    if (!o["apps"].isArray() || o["apps"].toArray().isEmpty() || o["apps"].toArray().size() > 16 || !o["actions"].isArray() || o["actions"].toArray().isEmpty() || o["actions"].toArray().size() > 2 || !o["evidence"].isArray() || o["evidence"].toArray().size() > 16) return {};
    QSet<QString> handles, actions;
    for (auto app : o["apps"].toArray()) {
        if (!app.isObject()) return {};
        auto a = app.toObject();
        if (!keys(a,{"handle","identity_sha256","name","window"}) || !text(a["handle"],128) || handles.contains(a["handle"].toString()) || !text(a["name"],256) || !text(a["window"],512) || !text(a["identity_sha256"],64) || !hash.match(a["identity_sha256"].toString()).hasMatch()) return {};
        handles.insert(a["handle"].toString());
    }
    for (auto action : o["actions"].toArray()) {
        if (!action.isString() || (action != "ui.snapshot" && action != "ui.find") || actions.contains(action.toString())) return {};
        actions.insert(action.toString());
    }
    }
    return ScopePreview{o,o["digest"].toString(),expires};
}
ScopeDialog::ScopeDialog(const ScopePreview &preview) : preview_(preview) {
    setWindowTitle(tr("Minnerite — Needs permission"));
    setObjectName("aios-protected-confirmation");
    const bool termination=preview.document["kind"]=="process_termination";
    const bool powerProfile=preview.document["kind"]=="power_profile_change";
    setAccessibleName(termination ? tr("Minnerite process termination permission") : powerProfile ? tr("Minnerite power profile permission") : tr("Minnerite application read permission"));
    setModal(true); resize(600,540);
    auto layout = new QVBoxLayout(this);
    auto content = new QWidget; auto rows = new QVBoxLayout(content);
    auto label = [&](const QString &id, const QString &value) {
        auto w = new QLabel(value); w->setObjectName(id); w->setTextFormat(Qt::PlainText);
        w->setWordWrap(true); w->setTextInteractionFlags(Qt::TextSelectableByKeyboard | Qt::TextSelectableByMouse);
        w->setFocusPolicy(Qt::StrongFocus); w->setAccessibleName(value); rows->addWidget(w);
    };
    auto o = preview.document;
    label("lifecycle",tr("Needs permission"));
    label("target",tr("Target: %1\nDesktop: %2 · User: %3").arg(o["target"].toString(),o["session_id"].toString()).arg(quint64(o["uid"].toDouble())));
    label("profile",tr("Local / CPU: %1\nMode: %2").arg(o["profile"].toString(),o["mode"].toString()));
    label("goal",tr("Request:\n%1").arg(o["goal"].toString()));
    if (termination) {
        auto p=o["preview"].toObject();auto i=p["identity"].toObject();
        label("process",tr("Selected process: %1\nPID: %2 · User: %3\nStart ticks: %4\nBoot: %5\nExecutable identity: %6")
            .arg(o["process_id"].toString()).arg(quint64(i["pid"].toDouble())).arg(quint64(i["uid"].toDouble()))
            .arg(quint64(i["start_time_ticks"].toDouble())).arg(i["boot_id"].toString(),i["executable_identity"].toString()));
        label("closure",tr("System closure: %1").arg(o["closure"].toString()));
        label("scope",tr("Action: process.terminate — R2\nSend one SIGTERM to this process. Verify exit within %1 milliseconds.\nThis may interrupt work or lose unsaved data. It cannot be undone.\nAn ignored signal or timeout reports a partial effect. No automatic SIGKILL or retry.")
            .arg(p["verification_timeout_ms"].toInt()));
    } else if (powerProfile) {
        auto p=o["preview"].toObject();QStringList choices;for(auto value:p["available_profiles"].toArray())choices<<value.toString();
        label("scope",tr("Action: power.profile_set — R2\nChange power profile from %1 to %2.\nAvailable profiles: %3\nThe previous profile is recorded for recovery. The change is verified by native readback.")
            .arg(p["prior"].toString(),p["requested"].toString(),choices.join(", ")));
    } else {
    QStringList apps;
    for (auto a : o["apps"].toArray()) {
        auto v=a.toObject(); apps << tr("%1 — %2\nResource: %3\nIdentity: %4").arg(v["name"].toString(),v["window"].toString(),v["handle"].toString(),v["identity_sha256"].toString());
    }
    label("apps",tr("Selected application windows:\n%1").arg(apps.join("\n\n")));
    QStringList actions; for (auto a : o["actions"].toArray()) actions << a.toString();
    label("scope",tr("Read access: %1\nNo input or external effects are authorized by this read scope.").arg(actions.join(", ")));
    }
    QStringList evidence; for (auto e : o["evidence"].toArray()) evidence << e.toString();
    label("evidence",tr("Evidence: %1").arg(evidence.isEmpty() ? tr("None attached") : evidence.join(", ")));
    label("digest",tr("Proposal: %1").arg(preview.digest));
    auto expiry = new QLabel; expiry->setObjectName("expiry"); rows->addWidget(expiry);
    auto scroll = new QScrollArea; scroll->setWidgetResizable(true); scroll->setWidget(content); layout->addWidget(scroll);
    auto buttons = new QDialogButtonBox;
    auto cancel = buttons->addButton(tr("Cancel"),QDialogButtonBox::RejectRole);
    auto allow = buttons->addButton(termination ? tr("Terminate this process") : powerProfile ? tr("Change power profile") : tr("Allow this read scope"),QDialogButtonBox::AcceptRole);
    cancel->setObjectName("cancel"); allow->setObjectName("allow");
    cancel->setDefault(true); allow->setDefault(false); allow->setAutoDefault(false);
    cancel->setAccessibleDescription(tr("Decline and close this permission request"));
    allow->setAccessibleDescription(termination ? tr("Confirm one graceful termination attempt on only the displayed process") : powerProfile ? tr("Confirm only the displayed power profile change") : tr("Confirm only the displayed application read scope until its expiration"));
    layout->addWidget(buttons); cancel->setFocus();
    connect(cancel,&QPushButton::clicked,this,[this]{finish(false);});
    connect(allow,&QPushButton::clicked,this,[this]{finish(true);});
    connect(this,&QDialog::rejected,this,[this]{finish(false);});
    auto timer = new QTimer(this); timer->setInterval(50);
    connect(timer,&QTimer::timeout,this,[this,expiry]{
        auto now = boottimeMs();
        if (!now || now >= preview_.expires) { withdraw(); return; }
        expiry->setText(tr("Expires in %1 seconds. Stop or a changed proposal revokes permission.").arg((preview_.expires-now+999)/1000));
    }); timer->start();
}
void ScopeDialog::withdraw() { finish(false); }
void ScopeDialog::finish(bool allow) {
    if (decided_) return;
    decided_=true;
    auto now=boottimeMs();
    emit decision(preview_.digest,allow && now && now<preview_.expires);
    done(allow && now && now<preview_.expires ? QDialog::Accepted : QDialog::Rejected);
}
