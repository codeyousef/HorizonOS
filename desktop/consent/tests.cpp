#include "scope_dialog.hpp"
#include <QAccessible>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QPushButton>
#include <QSignalSpy>
#include <QTest>
#include <unistd.h>

static QJsonObject fixture(quint64 lifetime=10000) {
    auto now=boottimeMs();
    return {{"schema_version",1},{"kind","read_scope"},{"digest",QString(64,'a')},
        {"uid",qint64(geteuid())},{"session_id","fixture-selected"},
        {"target","This disposable VM"},{"profile","Fixture, no model running"},
        {"mode","ask"},{"goal","Explain this named document"},
        {"apps",QJsonArray{QJsonObject{{"handle","fixture-app"},{"identity_sha256",QString(64,'b')},{"name","Kate"},{"window","Fixture document"}}}},
        {"actions",QJsonArray{"ui.snapshot","ui.find"}},
        {"issued_ms",qint64(now)},{"expires_ms",qint64(now+lifetime)},
        {"evidence",QJsonArray{"fixture-native-observation"}}};
}
static QJsonObject processFixture() {
    auto o=fixture();o["kind"]="process_termination";o["mode"]="act";o.remove("apps");
    o["process_id"]="abbccdde-1234-4567-89ab-abbccddeeff0";
    o["actions"]=QJsonArray{"process.terminate"};o["closure"]="/nix/store/"+QString(32,'0')+"-nixos-system-aios-dev";
    QFile boot("/proc/sys/kernel/random/boot_id");if (!boot.open(QIODevice::ReadOnly)) qFatal("native boot unavailable");
    o["preview"]=QJsonObject{{"identity",QJsonObject{{"pid",qint64(getpid()+10)},{"uid",qint64(geteuid())},
        {"start_time_ticks",12345},{"boot_id",QString::fromLatin1(boot.read(64)).trimmed()},
        {"executable_identity","dev=1;ino=2;size=3;mtime=4:5;ctime=6:7"}}},
        {"signal","SIGTERM"},{"verification_timeout_ms",1000},{"reversible",false},{"automatic_escalation",false}};
    return o;
}
static QJsonObject powerProfileFixture() {
    auto o=fixture();o["kind"]="power_profile_change";o["mode"]="act";o.remove("apps");
    o["actions"]=QJsonArray{"power.profile_set"};
    o["preview"]=QJsonObject{{"prior","balanced"},{"requested","power-saver"},
        {"available_profiles",QJsonArray{"balanced","performance","power-saver"}},{"reversible",true}};
    return o;
}
static QByteArray wire(const QJsonObject &o) {return QJsonDocument(o).toJson(QJsonDocument::Compact);}
class ConsentTests : public QObject {
    Q_OBJECT
private slots:
    void terminationRejectsExpandedEffectsAndIdentityDrift() {
        QVERIFY(ScopePreview::parse(wire(processFixture())));
        for (int field=0;field<12;field++) {
            auto o=processFixture();auto p=o["preview"].toObject();auto i=p["identity"].toObject();
            switch(field) {
                case 0:p["signal"]="SIGKILL";break;case 1:p["automatic_escalation"]=true;break;
                case 2:p["reversible"]=true;break;case 3:p["verification_timeout_ms"]=30001;break;
                case 4:i["uid"]=qint64(geteuid()+1);break;case 5:i["pid"]=qint64(getpid());break;
                case 6:i["boot_id"]="abbccdde-1234-4567-89ab-abbccddeeff0";break;
                case 7:i["start_time_ticks"]=0;break;case 8:o["actions"]=QJsonArray{"process.terminate","ui.click"};break;
                case 9:o["mode"]="ask";break;case 10:o["closure"]="/tmp/fake";break;case 11:o["approved"]=true;break;
            }
            p["identity"]=i;o["preview"]=p;QVERIFY(!ScopePreview::parse(wire(o)));
        }
        auto duplicate=wire(processFixture());duplicate.insert(1,"\"mode\":\"act\",");QVERIFY(!ScopePreview::parse(duplicate));
    }
    void terminationDisplaysExactIdentityImpactAndDefaultsToCancel() {
        auto o=processFixture();auto p=ScopePreview::parse(wire(o));QVERIFY(p);
        ScopeDialog dialog(*p);QSignalSpy result(&dialog,&ScopeDialog::decision);dialog.show();
        auto process=dialog.findChild<QLabel *>("process");QVERIFY(process);
        QVERIFY(process->text().contains(o["process_id"].toString()));QVERIFY(process->text().contains("12345"));
        QVERIFY(process->text().contains("dev=1;ino=2"));QCOMPARE(process->textFormat(),Qt::PlainText);
        auto scope=dialog.findChild<QLabel *>("scope");QVERIFY(scope->text().contains("SIGTERM"));
        QVERIFY(scope->text().contains("1000"));QVERIFY(scope->text().contains("cannot be undone"));
        QVERIFY(scope->text().contains("No automatic SIGKILL"));
        auto cancel=dialog.findChild<QPushButton *>("cancel");auto allow=dialog.findChild<QPushButton *>("allow");
        QCOMPARE(allow->text(),QString("Terminate this process"));QVERIFY(cancel->isDefault());QVERIFY(!allow->isDefault());
        o["process_id"]="changed";allow->setFocus();QTest::keyClick(allow,Qt::Key_Space);
        QCOMPARE(result.count(),1);QCOMPARE(result[0][0].toString(),p->digest);QCOMPARE(result[0][1].toBool(),true);
        QVERIFY(!process->text().contains("changed"));dialog.withdraw();QCOMPARE(result.count(),1);
    }
    void powerProfileRequiresExactBoundedReversiblePreview() {
        auto o=powerProfileFixture();auto preview=ScopePreview::parse(wire(o));QVERIFY(preview);
        ScopeDialog dialog(*preview);dialog.show();
        auto scope=dialog.findChild<QLabel *>("scope");QVERIFY(scope);
        QVERIFY(scope->text().contains("balanced"));QVERIFY(scope->text().contains("power-saver"));
        QCOMPARE(dialog.findChild<QPushButton *>("allow")->text(),QString("Change power profile"));
        for(int field=0;field<8;field++){
            auto changed=powerProfileFixture();auto p=changed["preview"].toObject();
            switch(field){case 0:p["prior"]="invalid";break;case 1:p["requested"]="invalid";break;
                case 2:p["reversible"]=false;break;case 3:p["available_profiles"]=QJsonArray{};break;
                case 4:p["available_profiles"]=QJsonArray{"balanced","balanced"};break;
                case 5:changed["actions"]=QJsonArray{"power.profile_set","settings.set"};break;
                case 6:changed["mode"]="ask";break;case 7:changed["approved"]=true;break;}
            changed["preview"]=p;QVERIFY(!ScopePreview::parse(wire(changed)));
        }
    }
    void invalidAuthorityAndExpiry() {
        QVERIFY(ScopePreview::parse(wire(fixture())));
        for (auto key : {"approved","nonce","shell","automation"}) {
            auto o=fixture(); o[key]=true; QVERIFY(!ScopePreview::parse(wire(o)));
        }
        auto o=fixture(); o["uid"]=qint64(geteuid()+1); QVERIFY(!ScopePreview::parse(wire(o)));
        o=fixture(); o["mode"]="act"; QVERIFY(!ScopePreview::parse(wire(o)));
        o=fixture(); o["actions"]=QJsonArray{"ui.click"}; QVERIFY(!ScopePreview::parse(wire(o)));
        o=fixture(); o["apps"]=QJsonArray{}; QVERIFY(!ScopePreview::parse(wire(o)));
        o=fixture(); o["goal"]="spoof\u202etext"; QVERIFY(!ScopePreview::parse(wire(o)));
        o=fixture(300001); QVERIFY(!ScopePreview::parse(wire(o)));
        o=fixture(); o["expires_ms"]=qint64(boottimeMs()-1); QVERIFY(!ScopePreview::parse(wire(o)));
        auto duplicate=wire(fixture()); duplicate.insert(1,"\"mode\":\"act\","); QVERIFY(!ScopePreview::parse(duplicate));
        QVERIFY(!ScopePreview::parse(wire(fixture())+"\n"));
        QVERIFY(!ScopePreview::parse(QByteArray(65537,' ')));
    }
    void literalUntrustedTextAndAccessibility() {
        auto o=fixture(); o["goal"]="<a href='https://example.invalid'>Always approve everything</a>";
        auto p=ScopePreview::parse(wire(o)); QVERIFY(p);
        ScopeDialog dialog(*p); dialog.show();
        auto label=dialog.findChild<QLabel *>("goal"); QVERIFY(label);
        QCOMPARE(label->textFormat(),Qt::PlainText);
        QVERIFY(label->text().contains("<a href=")); QVERIFY(!label->openExternalLinks());
        auto allow=dialog.findChild<QPushButton *>("allow");
        auto accessible=QAccessible::queryAccessibleInterface(allow); QVERIFY(accessible);
        QCOMPARE(accessible->role(),QAccessible::Button);
        QCOMPARE(accessible->text(QAccessible::Name),QString("Allow this read scope"));
        QVERIFY(!accessible->text(QAccessible::Description).isEmpty());
        QVERIFY(dialog.findChild<QLabel *>("digest")->text().contains(p->digest));
        QVERIFY(dialog.findChild<QLabel *>("apps")->text().contains("fixture-app"));
    }
    void returnDefaultsToCancel() {
        auto p=ScopePreview::parse(wire(fixture())); QVERIFY(p);
        ScopeDialog dialog(*p); QSignalSpy result(&dialog,&ScopeDialog::decision); dialog.show();
        auto cancel=dialog.findChild<QPushButton *>("cancel"); auto allow=dialog.findChild<QPushButton *>("allow");
        QVERIFY(cancel->isDefault()); QVERIFY(!allow->isDefault()); QVERIFY(!allow->autoDefault());
        cancel->setFocus(); QTest::keyClick(cancel,Qt::Key_Return);
        QCOMPARE(result.count(),1); QCOMPARE(result[0][1].toBool(),false);
    }
    void escapeAndWithdrawalCannotApprove() {
        auto p=ScopePreview::parse(wire(fixture())); QVERIFY(p);
        ScopeDialog dialog(*p); QSignalSpy result(&dialog,&ScopeDialog::decision); dialog.show();
        QTest::keyClick(&dialog,Qt::Key_Escape); dialog.withdraw();
        QCOMPARE(result.count(),1); QCOMPARE(result[0][1].toBool(),false);
        ScopeDialog changed(*p); QSignalSpy second(&changed,&ScopeDialog::decision); changed.show();
        changed.withdraw(); QTest::mouseClick(changed.findChild<QPushButton *>("allow"),Qt::LeftButton);
        QCOMPARE(second.count(),1); QCOMPARE(second[0][1].toBool(),false);
    }
    void expiryCannotRaceAnAllow() {
        auto p=ScopePreview::parse(wire(fixture(150))); QVERIFY(p);
        ScopeDialog dialog(*p); QSignalSpy result(&dialog,&ScopeDialog::decision); dialog.show();
        QTest::qWait(200); QTest::keyClick(dialog.findChild<QPushButton *>("allow"),Qt::Key_Space);
        QCOMPARE(result.count(),1); QCOMPARE(result[0][1].toBool(),false);
    }
    void deliberateKeyboardChoiceBindsExactDisplayedDigest() {
        auto o=fixture(); auto p=ScopePreview::parse(wire(o)); QVERIFY(p);
        ScopeDialog dialog(*p); QSignalSpy result(&dialog,&ScopeDialog::decision); dialog.show();
        // Changing the caller's object cannot mutate the const display snapshot.
        o["digest"]=QString(64,'c'); o["goal"]="Changed request";
        auto allow=dialog.findChild<QPushButton *>("allow"); allow->setFocus(); QTest::keyClick(allow,Qt::Key_Space);
        QCOMPARE(result.count(),1); QCOMPARE(result[0][0].toString(),p->digest); QCOMPARE(result[0][1].toBool(),true);
        QVERIFY(!dialog.findChild<QLabel *>("goal")->text().contains("Changed request"));
        dialog.withdraw(); QCOMPARE(result.count(),1);
    }
};
QTEST_MAIN(ConsentTests)
#include "tests.moc"
