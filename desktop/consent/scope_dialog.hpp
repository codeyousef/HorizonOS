#pragma once
#include <QDialog>
#include <QJsonObject>
#include <QString>
#include <optional>

// A display snapshot, never a capability or a model-owned plan. Only the
// supervising broker can interpret a response and revalidate native bindings.
struct ScopePreview {
    QJsonObject document;
    QString digest;
    quint64 expires;
    static std::optional<ScopePreview> parse(const QByteArray &canonical);
};
quint64 boottimeMs();
class ScopeDialog final : public QDialog {
    Q_OBJECT
public:
    explicit ScopeDialog(const ScopePreview &preview);
    void withdraw();
signals:
    void decision(QString digest, bool allow);
private:
    const ScopePreview preview_;
    bool decided_ = false;
    void finish(bool allow);
};
