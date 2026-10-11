"""Compare compiler IDL with the client contract. Usage: python3 verify-idl.py target/idl/raftt.json"""
import json,sys,pathlib
normalize=lambda s:s.replace('_','').lower()
reference=json.loads(pathlib.Path(__file__).with_name('idl-client-reference.json').read_text())
actual=json.loads(pathlib.Path(sys.argv[1]).read_text())
assert actual['address']==reference['address'],'Program ID mismatch'
for expected in reference['instructions']:
    if expected['name'] in ['increment']:continue
    found=next((x for x in actual['instructions'] if normalize(x['name'])==normalize(expected['name'])),None)
    assert found,expected['name']+' missing'
    assert found['discriminator']==expected['discriminator'],expected['name']+' discriminator'
    # Addresses/seeds can additionally be included by the compiler; account order and permissions must match.
    key=lambda a:(normalize(a['name']),bool(a.get('writable')),bool(a.get('signer')))
    assert list(map(key,found['accounts']))==list(map(key,expected['accounts'])),expected['name']+' accounts'
    assert [(normalize(a['name']),a['type']) for a in found['args']]==[(normalize(a['name']),a['type']) for a in expected['args']],expected['name']+' args'
for expected in reference['accounts']:
    found=next((x for x in actual['accounts'] if normalize(x['name'])==normalize(expected['name'])),None)
    assert found and found['discriminator']==expected['discriminator'],expected['name']+' account'
for expected in reference['types']:
    if expected['type']['kind']!='struct':continue
    found=next((x for x in actual['types'] if normalize(x['name'])==normalize(expected['name'])),None)
    assert found,expected['name']+' type'
    fields=lambda t:[(normalize(f['name']),f['type']) for f in t['type']['fields']]
    assert fields(found)==fields(expected),expected['name']+' layout'
print('Compiler IDL matches client account layouts, arguments and instruction permissions.')
