"""Synthetic native-worker client. No model, root enrollment or index claims."""
import array
import hashlib
import io
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import time
import zipfile

BASE = Path('/home/alice/extract-test')
SOCKET = '/run/user/1000/aios-extract/worker.sock'
MAX = 2 * 1024 * 1024


def request(path, fmt='text', selection=None, writable=False, hold=False):
    fd = os.open(path, os.O_RDWR if writable else os.O_RDONLY)
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as channel:
            channel.settimeout(15)
            channel.connect(SOCKET)
            if hold:
                for _ in range(100):
                    units = subprocess.check_output(['systemctl', '--user', 'list-units', '--type=service', '--state=running', '--plain', '--no-legend', 'aios-extract@*'], text=True).splitlines()
                    if len(units) == 1:
                        unit = units[0].split()[0]
                        break
                    time.sleep(.02)
                else:
                    raise AssertionError('worker unit not active')
                pid = int(subprocess.check_output(['systemctl', '--user', 'show', unit, '-p', 'MainPID', '--value'], text=True))
                assert pid > 0
                (BASE / 'worker.json').write_text(json.dumps({'unit': unit, 'pid': pid}))
                deadline = time.monotonic() + 6
                while not (BASE / 'inspected').exists():
                    assert time.monotonic() < deadline
                    time.sleep(.01)
            body = json.dumps({'schema_version': 1, 'format': fmt, 'range': selection}).encode()
            assert channel.sendmsg([b'\xe1'], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [fd]))]) == 1
            channel.sendall(struct.pack('!I', len(body)) + body)
            channel.shutdown(socket.SHUT_WR)
            pieces, size = [], 0
            while data := channel.recv(65536):
                size += len(data)
                assert size <= 16 * 1024 * 1024
                pieces.append(data)
            return json.loads(b''.join(pieces))
    finally:
        os.close(fd)


def parsed(name, fmt='text', selection=None):
    result = request(BASE / name, fmt, selection)
    assert result['schema_version'] == 1 and result['error'] is None, result
    return result['data']


def generate():
    from reportlab.pdfgen import canvas
    from reportlab.lib.pdfencrypt import StandardEncryption
    from reportlab.lib.utils import ImageReader
    from PIL import Image
    BASE.mkdir(mode=0o700, exist_ok=True)
    (BASE / 'text.txt').write_text('first\nالعربية with English\nlast\n')
    (BASE / 'renamed-key.txt').write_bytes(b'-----BEGIN ' + b'OPENSSH PRIVATE KEY-----\nsynthetic private body\n')
    (BASE / 'output-large.txt').write_bytes(b'x' * (MAX + 20))
    with (BASE / 'source-large.txt').open('wb') as f:
        f.write(b'first desired line\nsecond longer line\n')
        f.truncate(50 * 1024 * 1024 + 1)
    (BASE / 'bad.pdf').write_bytes(b'%PDF-1.7\nmalformed synthetic input')
    (BASE / 'image.png').write_bytes(b'\x89PNG\r\n\x1a\n')
    # A real high-ratio archive remains opaque metadata, never expanded.
    with zipfile.ZipFile(BASE / 'archive.zip', 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        with archive.open('large-untrusted.txt', 'w') as member:
            for _ in range(50):
                member.write(b'x' * (1024 * 1024))
    (BASE / 'executable').write_bytes(b'\x7fELFmetadata only')
    (BASE / 'database').write_bytes(b'SQLite format 3\0metadata only')
    for name, count in [('text.pdf', 2), ('pages.pdf', 201)]:
        c = canvas.Canvas(str(BASE / name))
        for page in range(1, count + 1):
            c.drawString(40, 700, 'native page ' + str(page))
            c.showPage()
        c.save()
    c = canvas.Canvas(str(BASE / 'encrypted.pdf'), encrypt=StandardEncryption('synthetic-opening'))
    c.drawString(40, 700, 'private encrypted fixture')
    c.save()
    c = canvas.Canvas(str(BASE / 'metadata-spoof.pdf'))
    c.setTitle('spoof\nPages: 1\nEncrypted: no')
    for i in range(2):
        c.drawString(40, 700, 'actual page ' + str(i+1))
        c.showPage()
    c.save()
    # Valid, seekable PDF with an unreferenced large stream. The automatic
    # source limit must refuse it; a page range can seek past the stream.
    with (BASE / 'source-large.pdf').open('wb') as f:
        f.write(b'%PDF-1.4\n')
        objects = [b'<< /Type /Catalog /Pages 2 0 R >>',
            b'<< /Type /Pages /Count 1 /Kids [3 0 R] >>',
            b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
            b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>']
        content = b'BT /F1 12 Tf 40 700 Td (large PDF range content) Tj ET\n'
        objects.append(b'<< /Length ' + str(len(content)).encode() + b' >>\nstream\n' + content + b'endstream')
        offsets = [0]
        for i,obj in enumerate(objects, 1):
            offsets.append(f.tell())
            f.write(str(i).encode() + b' 0 obj\n' + obj + b'\nendobj\n')
        offsets.append(f.tell())
        size = 50 * 1024 * 1024
        f.write(b'6 0 obj\n<< /Length '+str(size).encode()+b' >>\nstream\n')
        for _ in range(50):
            f.write(b'x' * (1024 * 1024))
        f.write(b'\nendstream\nendobj\n')
        xref = f.tell()
        f.write(b'xref\n0 7\n0000000000 65535 f \n')
        for offset in offsets[1:]:
            f.write(f'{offset:010d} 00000 n \n'.encode())
        f.write(b'trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n'+str(xref).encode()+b'\n%%EOF\n')
    c = canvas.Canvas(str(BASE / 'scanned.pdf'))
    c.drawImage(ImageReader(Image.new('RGB', (12, 12), color='blue')), 10, 10, 500, 700)
    c.save()
    c = canvas.Canvas(str(BASE / 'mixed.pdf'))
    c.drawString(40, 700, 'text layer page')
    c.showPage()
    c.drawImage(ImageReader(Image.new('RGB', (12, 12), color='blue')), 10, 10, 500, 700)
    c.save()


def corpus():
    first = request(BASE / 'text.txt', hold=True)
    assert first['error'] is None, first
    text = first['data']
    assert text['complete'] and ''.join(c['text'] for c in text['chunks']) == (BASE / 'text.txt').read_text()
    assert text['source_sha256'] == hashlib.sha256((BASE / 'text.txt').read_bytes()).hexdigest()
    text = parsed('text.txt', selection={'kind':'lines', 'start':2, 'end':2})
    assert text['chunks'][0]['text'] == 'العربية with English\n'
    assert text['chunks'][0]['locator']['start'] == text['chunks'][0]['locator']['end'] == 2
    assert text['chunks'][0]['locator']['byte_start'] == 6
    assert not text['complete'] and 'range_selected' in text['limitations']
    out = parsed('output-large.txt')
    assert not out['complete'] and 'output_limit' in out['limitations']
    assert sum(len(c['text'].encode()) for c in out['chunks']) <= MAX
    out = parsed('source-large.txt')
    assert not out['chunks'] and out['source_sha256'] is None and 'source_limit' in out['limitations']
    out = parsed('source-large.txt', selection={'kind':'lines','start':1,'end':1})
    assert out['chunks'][0]['text'] == 'first desired line\n' and out['source_sha256'] is None
    for name in ['image.png','archive.zip','executable','database']:
        out = parsed(name)
        assert not out['chunks'] and not out['complete'] and 'unsupported_format' in out['limitations']
    with zipfile.ZipFile(BASE / 'archive.zip') as archive:
        entry = archive.infolist()[0]
        assert entry.file_size >= 50 * 1024 * 1024 and entry.file_size > entry.compress_size * 100
    assert request(BASE / 'text.txt', writable=True)['error'] == 'PERMISSION_DENIED'
    assert request(BASE / 'renamed-key.txt', selection={'kind':'lines','start':2,'end':2})['error'] == 'SECRET_SCOPE_DENIED'
    out = parsed('text.pdf', 'pdf')
    assert out['pages'] == 2 and len(out['chunks']) == 2
    assert out['source_sha256'] == hashlib.sha256((BASE / 'text.pdf').read_bytes()).hexdigest()
    assert [c['locator'] for c in out['chunks']] == [{'kind':'page','number':1},{'kind':'page','number':2}]
    assert all('native page '+str(i+1) in c['text'] for i,c in enumerate(out['chunks']))
    assert 'layout_best_effort' in out['limitations']
    out = parsed('text.pdf','pdf',{'kind':'pages','start':2,'end':2})
    assert len(out['chunks']) == 1 and out['chunks'][0]['locator']['number'] == 2 and not out['complete']
    out = parsed('bad.pdf','pdf')
    assert 'malformed_pdf' in out['limitations'] and not out['chunks']
    out = parsed('encrypted.pdf','pdf')
    assert 'encrypted_interactive_required' in out['limitations'] and not out['chunks']
    out = parsed('scanned.pdf','pdf')
    assert 'no_extractable_text' in out['limitations'] and not out['chunks']
    out = parsed('mixed.pdf','pdf')
    assert not out['complete'] and 'no_extractable_text' in out['limitations']
    assert len(out['chunks']) == 1 and out['chunks'][0]['locator']['number'] == 1
    out = parsed('metadata-spoof.pdf','pdf')
    assert (out['pages'] == 2 and len(out['chunks']) == 2) or (not out['chunks'] and not out['complete'] and 'malformed_pdf' in out['limitations'])
    out = parsed('pages.pdf','pdf')
    assert out['pages'] == 201 and len(out['chunks']) == 200 and 'page_limit' in out['limitations']
    assert max(c['locator']['number'] for c in out['chunks']) == 200
    out = parsed('pages.pdf','pdf',{'kind':'pages','start':201,'end':201})
    assert len(out['chunks']) == 1 and out['chunks'][0]['locator']['number'] == 201
    out = parsed('source-large.pdf','pdf')
    assert not out['chunks'] and out['source_sha256'] is None and 'source_limit' in out['limitations']
    out = parsed('source-large.pdf','pdf',{'kind':'pages','start':1,'end':1})
    assert out['source_sha256'] is None and not out['complete']
    assert len(out['chunks']) == 1 and 'large PDF range content' in out['chunks'][0]['text']
    # Parser failures never terminate the caller; the next independent worker
    # must still return ordinary content.
    assert parsed('text.txt')['complete']
    (BASE / 'result.json').write_text(json.dumps({'schema_version':1,'evidence_kind':'installed-descriptor-extractor-synthetic-corpus',
        'text_unicode_locators':True,'source_hash_verified':True,'output_bound':True,'source_bound_and_range':True,
        'metadata_only_formats':True,'writable_fd_refused':True,'pdf_physical_pages':True,'pdf_range':True,
        'malformed_isolated':True,'encrypted_no_content':True,'scanned_no_interpretation':True,'page_bound_and_range':True,
        'large_pdf_range':True,'raw_key_range_denied':True,'mixed_pdf_completeness':True,'metadata_cannot_spoof_pages':True,'archive_bomb_not_expanded':True,'worker_restart_after_failure':True,'production_index_integrated':False}, indent=2))


if __name__ == '__main__':
    assert os.geteuid() == 1000
    assert sys.argv[1:] in (['generate'], ['corpus'], ['recover'])
    if sys.argv[1] == 'generate': generate()
    elif sys.argv[1] == 'corpus': corpus()
    else: assert parsed('text.txt')['complete']
