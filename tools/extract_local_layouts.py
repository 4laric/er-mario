# Local-only selective archive reader, following er-mario src/assets/archive.rs.
# Reads vanilla archive indexes and writes ONLY the two requested sprite-layout containers.
import argparse,pathlib,re,struct
parser=argparse.ArgumentParser(description='Extract only sprite layouts from your own Elden Ring installation.')
parser.add_argument('--game',required=True,type=pathlib.Path)
parser.add_argument('--output',required=True,type=pathlib.Path)
args=parser.parse_args()
from cryptography.hazmat.primitives.serialization import load_pem_public_key
from cryptography.hazmat.primitives.ciphers import Cipher,algorithms,modes
game=args.game
out=args.output
keys=[load_pem_public_key(p).public_numbers() for p in re.findall(rb'-----BEGIN RSA PUBLIC KEY-----.*?-----END RSA PUBLIC KEY-----', (game/'eldenring.exe').read_bytes(),re.S)]
def hashpath(p):
    h=0
    for c in p.lower().encode():h=(h*0x85+c)&((1<<64)-1)
    return h
wanted={hashpath('/menu/'+q+'/01_common.sblytbnd.dcx'):q for q in ['hi','low']}
def decode(key,data):
    k=(key.n.bit_length()+7)//8
    return b''.join(pow(int.from_bytes(data[i:i+k],'big'),key.e,key.n).to_bytes(k-1,'big') for i in range(0,len(data),k))
for archive in ['Data0','Data1','Data2','Data3','DLC']:
    if not wanted:break
    bhd=game/(archive+'.bhd')
    if not bhd.exists():continue
    encrypted=bhd.read_bytes()
    key=next((k for k in keys if decode(k,encrypted[:(k.n.bit_length()+7)//8]).startswith(b'BHD5')),None)
    assert key is not None,archive
    index=decode(key,encrypted)
    count,buckets=struct.unpack_from('<ii',index,16)
    for b in range(count):
        n,off=struct.unpack_from('<ii',index,buckets+b*8)
        for i in range(n):
            at=off+i*40
            h,padded,size,offset=struct.unpack_from('<Qiiq',index,at)
            if h not in wanted:continue
            aesoff=struct.unpack_from('<q',index,at+32)[0]
            with (game/(archive+'.bdt')).open('rb') as f:
                f.seek(offset);data=bytearray(f.read(padded))
            assert len(data)==padded
            if aesoff:
                aeskey=index[aesoff:aesoff+16]
                ranges=struct.unpack_from('<i',index,aesoff+16)[0]
                for r in range(ranges):
                    start,end=struct.unpack_from('<qq',index,aesoff+20+r*16)
                    if start<0 or end<=start:continue
                    end=min(end,len(data));end=start+((end-start)//16)*16
                    decoder=Cipher(algorithms.AES(aeskey),modes.ECB()).decryptor()
                    data[start:end]=decoder.update(bytes(data[start:end]))+decoder.finalize()
            if size:data=data[:size]
            q=wanted.pop(h);dest=out/q/'01_common.sblytbnd.dcx';dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
            print('Extracted ONLY layout:',q,'archive',archive,'bytes',len(data),flush=True)
assert not wanted,'requested layouts missing'
