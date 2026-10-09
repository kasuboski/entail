#!/usr/bin/env python3
"""Generate Author v0 benchmark models and measure a local CLI/worker honestly."""
from __future__ import annotations
import argparse, json, os, platform, shutil, statistics, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "benchmarks"
def ref(name): return {"kind":"ref", "path":f"input.{name}"}
def integer(n, unit="Count"): return {"kind":"int", "value":str(n), "unit":unit}
def origin(note="benchmark-generated"): return {"kind":"proposal", "source_ids":[], "review":"proposed", "note":note}
def model(mid):
    return dict(language_version="author-model/0.1", model_id=mid, sources=[], enums=[], inputs=[], derived=[], outcomes=[], domain=[], assumptions=[], rules=[], constraints=[], open_items=[])
def decl(id, typ): return {"id":id,"type":typ,"origin":origin()}
def rule(id, expr, outcome="Grant"): return {"id":id,"when":expr,"then":{"outcome":outcome,"fields":{}},"origin":origin()}
def comparison(kind, left, right): return {"kind":kind,"left":left,"right":right}
def make_threshold(n):
    m=model(f"Threshold{n}"); m['inputs']=[decl('x',{"kind":"int","unit":"Count"})]; m['outcomes']=[{"id":"Grant","fields":[],"origin":origin()}]
    m['rules']=[rule(f"R{i}",comparison("ge",ref('x'),integer(i))) for i in range(n)]
    return m
def generate():
    OUT.mkdir(exist_ok=True)
    cases={f"independent_thresholds_{n}":make_threshold(n) for n in (10,100,1000)}
    # Families isolate modeled structures; sizes are recorded in model IDs/files.
    m=make_threshold(40); m['model_id']='overlapping_guards'; m['rules']=[rule(f"R{i}",comparison("ge",ref('x'),integer(i))) for i in range(40)]; cases['overlapping_guards']=m
    m=make_threshold(40); m['model_id']='same_constructor_payload'; m['inputs'].append(decl('payload',{"kind":"int","unit":"Count"})); m['outcomes']=[{"id":"Grant","fields":[{"id":"value","type":{"kind":"int","unit":"Count"}}],"origin":origin()}]; m['rules']=[{"id":f"R{i}","when":comparison("ge",ref('x'),integer(i)),"then":{"outcome":"Grant","fields":{"value":ref('payload')}},"origin":origin()} for i in range(40)]; cases['same_constructor_payload']=m
    m=model('coupled_boolean'); m['inputs']=[decl(f"b{i}",{"kind":"bool"}) for i in range(24)]; m['outcomes']=[{"id":"Grant","fields":[],"origin":origin()}]; m['rules']=[rule('R0',{"kind":"and","args":[comparison('eq',ref(f'b{i}'),{"kind":"bool","value":bool(i%2)}) for i in range(24)]})]; cases['coupled_boolean']=m
    m=model('shared_derived_dag'); m['inputs']=[decl('x',{"kind":"int","unit":"Count"})]; m['derived']=[{"id":f"d{i}","type":{"kind":"int","unit":"Count"},"expr":comparison('add',ref('x') if i==0 else {"kind":"ref","path":f"derived.d{i-1}"},integer(1)),"origin":origin()} for i in range(64)]; m['outcomes']=[{"id":"Grant","fields":[],"origin":origin()}]; m['rules']=[rule('R0',comparison('ge',{"kind":"ref","path":"derived.d63"},integer(100)))]; cases['shared_derived_dag']=m
    m=model('near_model_limits'); m['inputs']=[decl(f"x{i}",{"kind":"int","unit":"Count"}) for i in range(256)]; m['outcomes']=[{"id":"Grant","fields":[],"origin":origin()}]; m['rules']=[rule(f"R{i}",comparison('ge',ref(f"x{i%256}"),integer(10**30+i))) for i in range(2048)]; cases['near_model_limits']=m
    for name,obj in cases.items():
        p=OUT/f"{name}.json"; p.write_text(json.dumps(obj,indent=2)+'\n')
    # Repeat-edit seed and metadata-only variant support cache/provenance probes.
    p=OUT/'repeated_edit_seed.json'; p.write_text(json.dumps(make_threshold(10),indent=2)+'\n')
    meta=make_threshold(10); meta['rules'][0]['origin']['note']='metadata-only change'; (OUT/'metadata_change.json').write_text(json.dumps(meta,indent=2)+'\n')
    return cases

def validate(models):
    # Structural checks without non-stdlib dependencies; semantic compile belongs to engine.
    def walk(v):
        if isinstance(v,dict):
            if v.get('kind')=='int' and 'value' in v: assert isinstance(v.get('value'),str) and (v['value']=='0' or (v['value'].lstrip('-').isdigit() and not v['value'].startswith('0') and not v['value'].startswith('-0'))), v
            for x in v.values(): walk(x)
        elif isinstance(v,list):
            for x in v: walk(x)
    for name,m in models.items():
        assert m['language_version']=='author-model/0.1' and all(k in m for k in ('inputs','derived','outcomes','rules','constraints','open_items'))
        walk(m)

def percentile(values, p):
    return sorted(values)[min(len(values)-1, max(0, int((len(values)-1)*p)))] if values else None

def frame_write(proc, value):
    payload=json.dumps(value,separators=(',',':')).encode()
    proc.stdin.write(len(payload).to_bytes(4,'big')+payload); proc.stdin.flush()

def frame_read(proc):
    header=proc.stdout.read(4)
    if len(header)!=4: raise RuntimeError('worker EOF before frame header')
    size=int.from_bytes(header,'big'); payload=proc.stdout.read(size)
    if len(payload)!=size: raise RuntimeError('worker EOF in frame payload')
    return json.loads(payload)

def start_worker(binary, max_obligations):
    proc=subprocess.Popen([str(binary),'worker','--max-obligations',str(max_obligations)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    frame_write(proc,{'protocol_version':'author-worker/0.1','request_id':'hello','op':'hello','params':{}})
    hello=frame_read(proc)
    if 'error' in hello: raise RuntimeError(f'worker hello failed: {hello}')
    return proc,hello['result']
def request(proc, seq, op, params):
    frame_write(proc,{'protocol_version':'author-worker/0.1','request_id':str(seq),'op':op,'params':params})
    reply=frame_read(proc)
    if 'error' in reply: raise RuntimeError(f'{op} failed: {reply}')
    return reply['result']
def summarize(values):
    return {'n':len(values),'p50_ms':statistics.median(values) if values else None,'p95_ms':percentile(values,.95)}
def stats(report):
    results=report.get('results',[]); statuses={}
    for item in results: statuses[item.get('status','unknown')]=statuses.get(item.get('status','unknown'),0)+1
    return {'elapsed_ms':report.get('elapsed_ms'),'solver_query_count':report.get('solver_query_count'),'cache_hits':report.get('cache_hits'),'planned_obligation_count':report.get('planned_obligation_count'),'executed_obligation_count':report.get('executed_obligation_count'),'statuses':statuses,'inconclusive':sum(statuses.get(s,0) for s in ('unknown','timeout','cancelled','skipped','error'))}
def run_worker_family(binary,name,model,runs,budget,max_obligations):
    proc,hello=start_worker(binary,max_obligations); seq=0; compile_ms=[]; analyze_ms=[]; reports=[]; hashes=[]
    try:
        for i in range(runs):
            t=time.perf_counter(); compiled=request(proc,seq,'compile',{'model':model}); seq+=1; compile_ms.append((time.perf_counter()-t)*1000)
            if not compiled.get('valid'): raise RuntimeError(f'compile rejected {name}: {compiled}')
            hashes.append({'semantic_hash':compiled.get('semantic_hash'),'content_hash':compiled.get('content_hash')})
            t=time.perf_counter(); report=request(proc,seq,'analyze',{'handle':compiled['handle'],'checks':['rule_reachability','coverage','outcome_conflict'],'budget':budget}); seq+=1; analyze_ms.append((time.perf_counter()-t)*1000); reports.append(stats(report))
        return {'family':name,'compile_wall':summarize(compile_ms),'analysis_wall':summarize(analyze_ms),'analysis_report':reports,'hashes':hashes,'hello':hello}
    finally:
        proc.stdin.close(); proc.wait(timeout=10)

def run(args):
    cases=generate(); validate(cases)
    if not args.binary:
        print('Generated and structurally validated fixtures; no executable specified, so no measurements collected.'); return
    binary=Path(args.binary).expanduser().resolve()
    if not binary.exists(): raise SystemExit(f'binary not found: {binary}')
    names=list(cases) if args.full else ['independent_thresholds_10','independent_thresholds_100','independent_thresholds_1000','overlapping_guards','same_constructor_payload','coupled_boolean','shared_derived_dag','near_model_limits']
    budget={'total_timeout_ms':args.budget_ms,'per_obligation_timeout_ms':args.per_obligation_ms,'max_obligations':args.max_obligations}
    env=os.environ.copy(); report={'platform':platform.platform(),'machine':platform.machine(),'processor':platform.processor() or 'unreported','python':platform.python_version(),'binary':str(binary),'profile':args.profile or 'unreported','runs_per_family':args.runs,'effective_budget':budget,'worker_max_obligations':args.max_obligations,'worker':[],'cli':[],'cache_probe':None,'unavailable_metrics':['peak memory (not sampled; use external RSS tooling)','formula size (statistics hook not yet exposed)','native per-obligation solver statistics (pending backend hook)']}
    try: report['rust_version']=subprocess.check_output(['rustc','--version'],text=True,stderr=subprocess.STDOUT,env=env).strip()
    except (OSError,subprocess.CalledProcessError): report['rust_version']='unavailable'
    for key in names:
        try: report['worker'].append(run_worker_family(binary,key,cases[key],args.runs,budget,args.max_obligations))
        except Exception as e: report['worker'].append({'family':key,'error':str(e)})
    # Explicit probes: duplicate obligations within one analysis expose planner/cache stats;
    # edits and assumption changes are isolated in subsequent requests/compiled handles.
    probe=model('cache_probe'); probe['inputs']=[decl('x',{'kind':'int','unit':'Count'})]; probe['outcomes']=[{'id':'Grant','fields':[],'origin':origin()}]; probe['rules']=[rule('R0',comparison('ge',ref('x'),integer(0)))]; probe['constraints']=[{'id':f'C{i}','for_outcome':'Grant','must':comparison('ge',ref('x'),integer(0)),'origin':origin()} for i in range(4)]
    proc,hello=start_worker(binary,args.max_obligations); seq=0
    try:
        compiled=request(proc,seq,'compile',{'model':probe}); seq+=1; handle=compiled['handle']
        base={'handle':handle,'checks':['constraint_compliance'],'budget':budget}
        repeated=[]
        for _ in range(args.runs): repeated.append(stats(request(proc,seq,'analyze',base))); seq+=1
        edited=json.loads(json.dumps(probe)); edited['rules'][0]['when']=comparison('ge',ref('x'),integer(1))
        edited_c=request(proc,seq,'compile',{'model':edited}); seq+=1
        metadata=json.loads(json.dumps(probe)); metadata['rules'][0]['origin']['note']='metadata only'; metadata['open_items']=[{'id':'I1','text':'provenance probe','source_ids':[],'affects':'interpretation'}]
        metadata_c=request(proc,seq,'compile',{'model':metadata}); seq+=1
        assumptions=json.loads(json.dumps(probe)); assumptions['assumptions']=[{'id':'A1','expr':comparison('ge',ref('x'),integer(0)),'origin':{'kind':'assumption','source_ids':[],'review':'proposed'}}]
        assumption_c=request(proc,seq,'compile',{'model':assumptions}); seq+=1
        assumption_report=request(proc,seq,'analyze',{'handle':assumption_c['handle'],'checks':['constraint_compliance'],'assumption_ids':['A1'],'budget':budget}); seq+=1
        report['cache_probe']={'same_analysis_reports':repeated,'semantic_edit_hashes':{'base':compiled.get('semantic_hash'),'edited':edited_c.get('semantic_hash')},'metadata_edit_hashes':{'base_semantic':compiled.get('semantic_hash'),'metadata_semantic':metadata_c.get('semantic_hash'),'base_content':compiled.get('content_hash'),'metadata_content':metadata_c.get('content_hash')},'assumption_config_report':stats(assumption_report),'expectations':{'semantic_edit_changes_semantic_hash':compiled.get('semantic_hash')!=edited_c.get('semantic_hash'),'metadata_edit_preserves_semantics':compiled.get('semantic_hash')==metadata_c.get('semantic_hash'),'metadata_edit_changes_content':compiled.get('content_hash')!=metadata_c.get('content_hash'),'assumptions_use_distinct_analysis_configuration':True}}
    except Exception as e: report['cache_probe']={'error':str(e)}
    finally: proc.stdin.close(); proc.wait(timeout=10)
    if args.cli:
        for key in names:
            times=[]; outs=[]
            for _ in range(args.runs):
                t=time.perf_counter(); p=subprocess.run([str(binary),'analyze',str(OUT/f'{key}.json'),'--checks','rule_reachability,coverage,outcome_conflict','--budget-ms',str(args.budget_ms)],capture_output=True,text=True,timeout=args.timeout,env=env); times.append((time.perf_counter()-t)*1000)
                try: outs.append(stats(json.loads(p.stdout)))
                except ValueError: outs.append({'error':p.stderr[:300]})
            report['cli'].append({'family':key,'wall':summarize(times),'reports':outs})
    print(json.dumps(report,indent=2))

def main():
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('--binary'); p.add_argument('--runs',type=int,default=5); p.add_argument('--full',action='store_true'); p.add_argument('--timeout',type=float,default=30); p.add_argument('--profile'); p.add_argument('--budget-ms',type=int,default=1500); p.add_argument('--per-obligation-ms',type=int,default=200); p.add_argument('--max-obligations',type=int,default=24); p.add_argument('--cli',action='store_true'); a=p.parse_args()
    if a.runs<5: p.error('measurement runs must be at least five');
    if a.binary: run(a)
    else: cases=generate(); validate(cases); print('Generated and structurally validated fixtures; no measurements collected.')
if __name__=='__main__': main()
