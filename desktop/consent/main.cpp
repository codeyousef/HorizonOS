#include "scope_dialog.hpp"
#include <QApplication>
#include <QJsonDocument>
#include <QSocketNotifier>
#include <QTimer>
#include <fcntl.h>
#include <sys/socket.h>
#include <unistd.h>
#include <memory>

int main(int argc, char **argv) {
    // No command line proposal, public bus approval method or automated accept
    // switch. A single anonymous socket belongs to the supervising parent.
    ucred peer{}; socklen_t size=sizeof(peer);
    if (argc!=1 || geteuid()==0 || getsockopt(0,SOL_SOCKET,SO_PEERCRED,&peer,&size)!=0 || peer.uid!=geteuid() || peer.pid!=getppid() || peer.pid<=1) return 2;
    if (fcntl(0,F_SETFL,fcntl(0,F_GETFL)|O_NONBLOCK)<0) return 2;
    QApplication app(argc,argv);
    app.setApplicationName("Horizon OS confirmation");
    app.setDesktopFileName("org.aios.Confirmation");
    app.setQuitOnLastWindowClosed(false);
    QByteArray input; std::unique_ptr<ScopeDialog> dialog;
    bool replied=false;
    auto reply = [&](QString digest,bool allow) {
        if (replied) return;
        replied=true;
        QByteArray output=QJsonDocument(QJsonObject{{"digest",digest},{"decision",allow ? "allow" : "cancel"}}).toJson(QJsonDocument::Compact)+"\n";
        auto sent=send(0,output.constData(),size_t(output.size()),MSG_NOSIGNAL);
        app.exit(sent==output.size() ? 0 : 3);
    };
    QTimer admission; admission.setSingleShot(true); admission.start(2000);
    QObject::connect(&admission,&QTimer::timeout,&app,[&]{app.exit(2);});
    QSocketNotifier reader(0,QSocketNotifier::Read);
    QObject::connect(&reader,&QSocketNotifier::activated,&app,[&]{
        char bytes[4096]; auto n=recv(0,bytes,sizeof(bytes),0);
        if (n<0 && (errno==EAGAIN || errno==EINTR)) return;
        if (n<=0 || dialog) {
            reader.setEnabled(false);
            if (dialog) dialog->withdraw(); else app.exit(2);
            return;
        }
        input.append(bytes,int(n));
        if (input.size()<4) return;
        const auto b=reinterpret_cast<const unsigned char *>(input.constData());
        quint32 length=(quint32(b[0])<<24)|(quint32(b[1])<<16)|(quint32(b[2])<<8)|b[3];
        if (!length || length>65536 || input.size()>qint64(length)+4) {app.exit(2);return;}
        if (input.size()!=qint64(length)+4) return;
        auto preview=ScopePreview::parse(input.mid(4));
        if (!preview) {app.exit(2);return;}
        admission.stop(); input.fill('\0');input.clear();
        dialog=std::make_unique<ScopeDialog>(*preview);
        QObject::connect(dialog.get(),&ScopeDialog::decision,&app,reply);
        dialog->show();
    });
    return app.exec();
}
