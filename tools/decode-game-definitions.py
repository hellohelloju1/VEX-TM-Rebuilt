"""Decode supplied .tmg2 resources without executing their embedded Python.
Usage: python tools/decode-game-definitions.py OUTPUT_DIRECTORY FILE.tmg2 ...
Requires google.protobuf only for this offline developer tool.
"""
import hashlib,json,sys,zlib
from pathlib import Path
from google.protobuf import descriptor_pb2,descriptor_pool,message_factory,json_format
root=Path(__file__).resolve().parents[1]
fd=json_format.Parse((root/'analysis/game-descriptor.json').read_text(),descriptor_pb2.FileDescriptorProto())
pool=descriptor_pool.DescriptorPool();pool.Add(fd)
Game=message_factory.GetMessageClass(pool.FindMessageTypeByName('Game'))
Wrapper=message_factory.GetMessageClass(pool.FindMessageTypeByName('GameWrapper'))
output=Path(sys.argv[1]);output.mkdir(parents=True,exist_ok=True)
for arg in sys.argv[2:]:
    p=Path(arg);raw=p.read_bytes()
    if len(raw)>16*1024*1024:raise ValueError('Resource too large')
    wrapper=Wrapper.FromString(raw)
    inflater=zlib.decompressobj();decoded=inflater.decompress(wrapper.data,16*1024*1024+1)
    if len(decoded)>16*1024*1024 or not inflater.eof:raise ValueError('Invalid or oversized game payload')
    game=Game.FromString(decoded)
    data=json_format.MessageToDict(game,preserving_proto_field_name=True)
    (output/(p.stem+'.json')).write_text(json.dumps(data,indent=2))
    if game.scoringModule:(output/(p.stem+'.py')).write_text(game.scoringModule)
    print(json.dumps({'file':p.name,'sha256':hashlib.sha256(raw).hexdigest(),'game':game.name,'season':game.season,'version':game.version}))
