#include "scope_dialog.hpp"
#include <QAccessible>
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
static QByteArray wire(const QJsonObject &o) {return QJsonDocument(o).toJson(QJsonDocument::Compact);}
class ConsentTests : public QObject {
    Q_OBJECT
private slots:
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
