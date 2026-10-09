#!/usr/bin/env python3
"""Generate Author v0 benchmark models and measure a local CLI/worker honestly."""
from __future__ import annotations
import argparse, json, os, platform, statistics, subprocess, sys, time
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

def run(args):
    cases=generate(); validate(cases)
    if not args.binary:
        print('Generated and structurally validated fixtures; no executable specified, so no measurements collected.')
        return
    binary=Path(args.binary).expanduser()
    if not binary.exists(): raise SystemExit(f"binary not found: {binary}")
    names=list(cases) if args.full else ['independent_thresholds_10','overlapping_guards','same_constructor_payload','coupled_boolean','shared_derived_dag']
    report={'platform':platform.platform(),'machine':platform.machine(),'processor':platform.processor() or 'unreported','python':platform.python_version(),'binary':str(binary),'profile':args.profile or 'unreported','rust_version':None,'z3_version':None,'runs':args.runs,'measurements':[],'unavailable_metrics':['peak memory','formula size','cache rate','inconclusive rate','query count','Rust/Z3 versions unless exposed by executable']}
    try: report['rust_version']=subprocess.check_output(['rustc','--version'],text=True,stderr=subprocess.STDOUT).strip()
    except (OSError,subprocess.CalledProcessError): pass
    for name in names:
        path=OUT/f'{name}.json'; samples=[]; failures=[]
        for _ in range(args.runs):
            start=time.perf_counter()
            # CLI contract may vary by integration; allow explicit args with {model} substitution.
            cmd=[part.replace('{model}',str(path)) for part in args.command]
            try: p=subprocess.run(cmd,capture_output=True,text=True,timeout=args.timeout,check=False)
            except subprocess.TimeoutExpired: failures.append('timeout'); continue
            samples.append((time.perf_counter()-start)*1000)
            if p.returncode: failures.append(f'exit {p.returncode}: {p.stderr[:300]}')
        report['measurements'].append({'family':name,'samples_ms':len(samples),'p50_wall_ms':statistics.median(samples) if samples else None,'p95_wall_ms':sorted(samples)[min(len(samples)-1,int(.95*len(samples)))] if samples else None,'failures':failures})
    print(json.dumps(report,indent=2))

def main():
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('--binary'); p.add_argument('--command',nargs='+',default=['{binary}','validate','{model}']); p.add_argument('--runs',type=int,default=3); p.add_argument('--full',action='store_true'); p.add_argument('--timeout',type=float,default=30); p.add_argument('--profile'); a=p.parse_args()
    if a.runs<1: p.error('--runs must be positive')
    if a.binary: a.command=[x.replace('{binary}',a.binary) for x in a.command]
    run(a)
if __name__=='__main__': main()
