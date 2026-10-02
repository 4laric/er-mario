#!/usr/bin/env python3
"""Package the optional Mario/AP companion from explicit public inputs only."""
import argparse,hashlib,json,pathlib,shutil,subprocess,zipfile

def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def git(root,*args): return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--world',required=True,type=pathlib.Path)
    p.add_argument('--dll',required=True,type=pathlib.Path)
    p.add_argument('--output',required=True,type=pathlib.Path)
    p.add_argument('--release',default='0.6.4')
    p.add_argument('--bridge-head',default='a39033e2e3f71b4fa6010defa4cc8f71b8e7baa5')
    p.add_argument('--ci-run',default='36953480616')
    p.add_argument('--artifact-id',default='11204034975')
    p.add_argument('--dll-sha256',default='8acf04b61bb75c44a75f45483a28d0b2b843a722d8af15d53479e11ead9cc289')
    a=p.parse_args();repo=pathlib.Path(__file__).resolve().parents[1];world=a.world.resolve()
    if git(repo,'status','--porcelain'): raise SystemExit('Mario source must be clean and committed before packaging.')
    world_blobs={}
    for name in ['tools/build_ap_icon.py','tools/ap_icon_src/ap_flower_160.bc7']:
        expected=git(world,'rev-parse','HEAD:'+name)
        actual=git(world,'hash-object','--path='+name,str(world/name))
        if actual!=expected: raise SystemExit('World public input differs from tracked HEAD: '+name)
        world_blobs[name]=expected
    if digest(a.dll)!=a.dll_sha256.lower(): raise SystemExit('DLL hash does not match the inspected CI artifact.')
    output=a.output.resolve();stage=output/'public'/'ER-Mario-AP'
    if stage.exists(): raise SystemExit('Public staging folder already exists; choose a new output folder or explicitly archive the old public stage.')
    stage.mkdir(parents=True)
    def copy(source,relative):
        target=stage/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,target)
    copy(a.dll,'er_mario.dll')
    for name in ['Compose-AP-Flower.ps1','Create-Paired-Profile.ps1','er-mario-setup.me3','er_mario.ini']:
        copy(repo/'distribution'/name,name)
    copy(repo/'distribution'/'MARIO-SETUP.md','README.md')
    copy(repo/'tools'/'extract_local_layouts.py','tools/extract_local_layouts.py')
    copy(world/'tools'/'build_ap_icon.py','tools/build_ap_icon.py')
    for name in ['ap_flower_160.bc7','README.md']:
        copy(world/'tools'/'ap_icon_src'/name,'tools/ap_icon_src/'+name)
    for source,relative in [(repo/'LICENSE','er-mario-MIT.txt'),(repo/'libsm64'/'LICENSE.md','libsm64-CC0.md'),
        (repo.parent/'fromsoftware-rs'/'LICENSE-MIT','fromsoftware-rs-MIT.txt'),
        (repo.parent/'fromsoftware-rs'/'LICENSE-ASL2','fromsoftware-rs-Apache-2.0.txt'),
        (world/'LICENSE','er-archipelago-MIT.txt')]:copy(source,'licenses/'+relative)
    for notice in (repo/'distribution'/'licenses').iterdir():
        if notice.is_file():copy(notice,'licenses/supplemental/'+notice.name)
    # Metadata comes from the checked-in lock, not a previous distribution's license list.
    meta=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1','--filter-platform','x86_64-pc-windows-msvc'],cwd=repo,text=True))
    nodes={n['id'] for n in meta['resolve']['nodes']};dependencies=[]
    for package in sorted(meta['packages'],key=lambda x:(x['name'],x['version'])):
        if package['source'] is None or package['id'] not in nodes:continue
        source=pathlib.Path(package['manifest_path']).parent
        texts=[f for f in source.iterdir() if f.is_file() and f.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE','UNLICENSE'))]
        if not texts: texts=[f for f in source.iterdir() if f.is_file() and f.name.upper().startswith('README')]
        folder=f"licenses/rust/{package['name']}-{package['version']}"
        for f in texts:copy(f,folder+'/'+f.name)
        dependencies.append({'name':package['name'],'version':package['version'],'license':package.get('license'),
            'source':package['source'],'notice_files':[f.name for f in texts],'supplemental_notice':('licenses/supplemental/'+package['name']+'-MIT.txt') if package['name'] in ['dasp_sample','vtable-rs-proc-macros'] else None})
    (stage/'licenses'/'rust-dependencies.json').write_text(json.dumps(dependencies,indent=2)+'\n')
    manifest={'schema':1,'release_companion_for':a.release,'upstream_mario_version':'0.3.3','bridge_abi':1,
        'bridge_source':a.bridge_head,'packaging_source':git(repo,'rev-parse','HEAD'),
        'packaging_source_dirty':bool(git(repo,'status','--porcelain')),'world_tool_source':git(world,'rev-parse','HEAD'),
        'world_tool_blobs':world_blobs,'fromsoftware_source':git(repo.parent/'fromsoftware-rs','rev-parse','HEAD'),
        'model_source':'n64decomp/sm64@06ec56df7f951f88da05f468cdcacecba496145a (texture-stripped mesh compiled in DLL)',
        'ci_run':'https://github.com/4laric/er-mario/actions/runs/'+a.ci_run,'ci_artifact_id':a.artifact_id,
        'private_asset_exclusions':['ROM','generated package','game archives','Oodle DLL','sprite layouts','generated menu atlases','logs','credentials'],
        'files':[{'path':str(f.relative_to(stage)).replace('\\','/'),'size':f.stat().st_size,'sha256':digest(f)} for f in sorted(stage.rglob('*')) if f.is_file()]}
    (stage/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    archive=output/f'ER-Mario-AP-v{a.release}.zip'
    with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
        for f in sorted(stage.rglob('*')):
            if f.is_file():
                info=zipfile.ZipInfo('ER-Mario-AP/'+str(f.relative_to(stage)).replace('\\','/'),date_time=(2026,10,2,0,0,0))
                info.compress_type=zipfile.ZIP_DEFLATED;info.external_attr=0o644<<16
                z.writestr(info,f.read_bytes(),compresslevel=9)
    print(json.dumps({'archive':str(archive),'sha256':digest(archive),'files':len(manifest['files'])+1,'private_assets_included':False},indent=2))
if __name__=='__main__':main()
