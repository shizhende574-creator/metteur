"""Frozen, bounded IDE repair acceptance; real provider through the actual host.

No automatic retry. Reuses only this phase's explicit USD1 / 40-request ledger.
Credentials remain in this Python process. Synthetic workspaces and copied binaries.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import threading
import uuid

from nx_model_eval import REPO, Relay, port, wait_listener, source_digest
from nx_model_support import Ledger, atomic_json, connection

SPECS = [
    {"id": "catalog_read", "kinds": ["Start", "ReadFile", "End"], "goal":
     "Create a blueprint named IU discovered read. Start must have an initial filePath attribute with literal evidence.txt; wire that output into ReadFile's actual path input, then End. The actual ReadFile output must contain the evidence text. No CallLLM, editing, commands, or execution tools."},
    {"id": "catalog_llm_branch", "kinds": ["Start", "CallLLM", "Contains", "Branch", "End"], "goal":
     "Create a blueprint named IU discovered LLM branch. Start -> CallLLM -> Branch, with two distinct End nodes for True and False. The CallLLM uses model flash, max_iterations 4 and max_tokens 512; its Prompt must ask to ReadFile evidence.txt and return the exact text without editing or executing anything. Contains must compare the CallLLM Result with the literal IU_REAL_SENTINEL_中文, and wire its boolean Result into Branch's actual condition input. Do not execute the blueprint yourself."},
    {"id": "manual_requirement", "manual": "HUMAN_REQUIREMENT_REAL_517: preserve and return the exact 中文 evidence text", "kinds": []},
]

class AuditedRelay(Relay):
    def __init__(self, config, ledger, receipt):
        super().__init__(config, ledger)
        self.receipt = receipt
        self.requests = []
    def transform(self, body):
        value = super().transform(body)
        text = json.dumps(value.get('messages', []), ensure_ascii=False)
        self.requests.append({"case": self.tag.get('case'), "stream": bool(value.get('stream')),
                              "manual_requirement_seen": SPECS[2]['manual'] in text,
                              "tool_names": [t.get('function', {}).get('name') for t in value.get('tools', [])]})
        atomic_json(self.receipt, self.requests)
        return value

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--credential-config', type=Path, required=True)
    p.add_argument('--budget-ledger', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--static', type=Path, required=True)
    p.add_argument('--node', type=Path, required=True)
    args=p.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    ledger=Ledger(args.budget_ledger, 1, max_calls=40)
    atomic_json(args.output/'frozen-specs.json', SPECS)
    relay=AuditedRelay(connection(args.credential_config, 'commandcode-flash'),ledger,args.output/'request-receipts.json')
    relay.tag={'variant':'candidate','mode':'ide','case':'setup'}
    threading.Thread(target=relay.serve_forever, daemon=True).start()
    dp,wp=port(),port()
    runtime=args.output/'bin'; runtime.mkdir()
    source=Path('D:/BaiduNetdiskDownload/metteur-a7-work/build/local-acceptance-native-target/debug')
    for name in ['metteurd.exe','metteur-web.exe']: shutil.copy2(source/name,runtime/name)
    config=(f'config_version = 2\n[daemon]\nautostart = "off"\nwake = false\n[llm]\ndefault_model = "flash"\nmax_retries = 0\nwrap_up_iterations = 1\nproject_instructions = false\n[llm.models.flash]\napi_type = "openai-chat"\napi_endpoint = "http://127.0.0.1:{relay.server_port}/v1"\napi_key = "{relay.fake_key}"\nmodel_id = "deepseek/deepseek-v4.1-flash"\ncontext_window_input = 64000\ncontext_window_output = 2048\nreplay_reasoning = true\n[sandbox]\nenabled = true\nmode = "sandbox"\nwhitelist = [\'node -e "process.exit(0)"\']\napproval_timeout_secs = 30\n[oversight]\nmode = "off"\n')
    (args.output/'config.toml').write_text(config,encoding='utf-8')
    summary={'spec_sha256':hashlib.sha256((args.output/'frozen-specs.json').read_bytes()).hexdigest(),'source_digest':source_digest(),'cases':[],'paid_budget_usd':1,'max_requests':40}
    children=[]; handles=[]
    try:
        for name,argv,wanted in [
            ('daemon',['--config',str(args.output/'config.toml'),'--data-dir',str(args.output/'data'),'--listen-addr',f'127.0.0.1:{dp}'],dp),
            ('web',['--daemon-addr',f'127.0.0.1:{dp}','--listen-addr',f'127.0.0.1:{wp}','--static-dir',str(args.static)],wp)]:
            log=(args.output/(name+'.log')).open('wb'); handles.append(log)
            proc=subprocess.Popen([str(runtime/('metteurd.exe' if name=='daemon' else 'metteur-web.exe')),*argv],stdout=log,stderr=log,creationflags=subprocess.CREATE_NO_WINDOW)
            children.append(proc); wait_listener(proc,wanted)
        for spec in SPECS:
            ledger.read()  # Unknowns and limits are enforced again on every request.
            root=args.output/spec['id'];root.mkdir();ws=root/'workspace';ws.mkdir()
            (ws/'evidence.txt').write_text('IU_REAL_SENTINEL_中文\nKeep exact evidence.\n',encoding='utf-8')
            ready={'workspace':ws.as_posix(),'baseURL':f'http://127.0.0.1:{wp}','model':'flash','case':spec}
            atomic_json(root/'ready.json',ready)
            relay.tag={'variant':'candidate','mode':'ide','case':spec['id']}
            proc=subprocess.run([str(args.node),str(REPO/'modules/webcore/scripts/ide-model-case.mjs'),str(root/'ready.json'),str(root/'report.json')],cwd=REPO/'modules/webcore',capture_output=True,timeout=360)
            (root/'driver.log').write_bytes(proc.stdout+proc.stderr)
            result=json.loads((root/'report.json').read_text(encoding='utf-8')) if (root/'report.json').exists() else {'status':'missing_receipt'}
            summary['cases'].append({'id':spec['id'],'exit_code':proc.returncode,'status':result['status'],'error':result.get('error'),'checks':result.get('checks')})
            atomic_json(args.output/'summary.json',summary)
            if proc.returncode: break  # Preserve failure, inspect before any retry.
    finally:
        for proc in reversed(children):
            proc.terminate()
            try:proc.wait(timeout=10)
            except subprocess.TimeoutExpired:proc.kill();proc.wait()
        for h in handles:h.close()
        relay.shutdown();relay.server_close()
        data=ledger.read()
        summary['request_count']=len(data['calls'])
        summary['upper_nano_usd']=sum(c.get('charged_upper_nano_usd',c['reserved_nano_usd']) for c in data['calls'])
        summary['states']=[c['state'] for c in data['calls']]
        atomic_json(args.output/'summary.json',summary)
        print(json.dumps(summary,ensure_ascii=True))

if __name__=='__main__':main()
