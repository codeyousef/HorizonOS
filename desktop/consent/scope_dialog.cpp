#include "scope_dialog.hpp"
#include <QDialogButtonBox>
#include <QJsonArray>
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
    if (!keys(o, {"schema_version","kind","digest","uid","session_id","target","profile","mode","goal","apps","actions","issued_ms","expires_ms","evidence"}) || o["schema_version"] != 1 || o["kind"] != "read_scope") return {};
    quint64 uid, issued, expires;
    auto now = boottimeMs();
    if (!integer(o["uid"], uid) || uid != geteuid() || uid == 0 || !integer(o["issued_ms"], issued) || !integer(o["expires_ms"], expires) || !now || issued > now || expires <= now || expires <= issued || expires-issued > 300000) return {};
    static const QRegularExpression hash("^[0-9a-f]{64}$");
    static const QRegularExpression session("^[A-Za-z0-9_-]{1,128}$");
    if (!text(o["digest"],64) || !hash.match(o["digest"].toString()).hasMatch() || !text(o["session_id"],128) || !session.match(o["session_id"].toString()).hasMatch() || !text(o["target"],256) || !text(o["profile"],256) || !text(o["goal"],4096) || (o["mode"] != "ask" && o["mode"] != "diagnose")) return {};
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
    for (auto e : o["evidence"].toArray()) if (!text(e,128)) return {};
    return ScopePreview{o,o["digest"].toString(),expires};
}
ScopeDialog::ScopeDialog(const ScopePreview &preview) : preview_(preview) {
    setWindowTitle(tr("Horizon OS — Needs permission"));
    setObjectName("aios-protected-confirmation");
    setAccessibleName(tr("Horizon OS application read permission"));
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
    label("target",tr("Target: %1\nDesktop: %2 · User: %3").arg(o["target"].toString(),o["session_id"].toString()).arg(o["uid"].toInt()));
    label("profile",tr("Local / CPU: %1\nMode: %2").arg(o["profile"].toString(),o["mode"].toString()));
    label("goal",tr("Request:\n%1").arg(o["goal"].toString()));
    QStringList apps;
    for (auto a : o["apps"].toArray()) {
        auto v=a.toObject(); apps << tr("%1 — %2\nResource: %3\nIdentity: %4").arg(v["name"].toString(),v["window"].toString(),v["handle"].toString(),v["identity_sha256"].toString());
    }
    label("apps",tr("Selected application windows:\n%1").arg(apps.join("\n\n")));
    QStringList actions; for (auto a : o["actions"].toArray()) actions << a.toString();
    label("scope",tr("Read access: %1\nNo input or external effects are authorized by this read scope.").arg(actions.join(", ")));
    QStringList evidence; for (auto e : o["evidence"].toArray()) evidence << e.toString();
    label("evidence",tr("Evidence: %1").arg(evidence.isEmpty() ? tr("None attached") : evidence.join(", ")));
    label("digest",tr("Proposal: %1").arg(preview.digest));
    auto expiry = new QLabel; expiry->setObjectName("expiry"); rows->addWidget(expiry);
    auto scroll = new QScrollArea; scroll->setWidgetResizable(true); scroll->setWidget(content); layout->addWidget(scroll);
    auto buttons = new QDialogButtonBox;
    auto cancel = buttons->addButton(tr("Cancel"),QDialogButtonBox::RejectRole);
    auto allow = buttons->addButton(tr("Allow this read scope"),QDialogButtonBox::AcceptRole);
    cancel->setObjectName("cancel"); allow->setObjectName("allow");
    cancel->setDefault(true); allow->setDefault(false); allow->setAutoDefault(false);
    cancel->setAccessibleDescription(tr("Decline and close this permission request"));
    allow->setAccessibleDescription(tr("Confirm only the displayed application read scope until its expiration"));
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
