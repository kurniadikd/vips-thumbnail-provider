import json

def apply_fixes():
    with open('error.json', 'r', encoding='utf-8') as f:
        lines = f.readlines()
        
    files_to_fix = {}
    
    for line in lines:
        try:
            msg = json.loads(line)
        except:
            continue
            
        if msg.get('reason') != 'compiler-message':
            continue
            
        message = msg.get('message', {})
        if message.get('code', {}).get('code') != 'E0308':
            continue
            
        spans = message.get('spans', [])
        primary_span = next((s for s in spans if s.get('is_primary')), None)
        if not primary_span:
            continue
            
        file_name = primary_span['file_name']
        line_start = primary_span['line_start'] - 1
        line_end = primary_span['line_end'] - 1
        col_start = primary_span['column_start'] - 1
        col_end = primary_span['column_end'] - 1
        
        expected = message.get('message', '')
        
        # We only care about expected i32 found i64/u64 etc, or expected u64 found u32 etc
        # Usually it's in the args. 
        if file_name not in files_to_fix:
            with open(file_name, 'r', encoding='utf-8') as src:
                files_to_fix[file_name] = src.readlines()
                
        # Fix strategy: we just append `.try_into().unwrap()` or `as i32` or `as usize` based on the error.
        text = files_to_fix[file_name][line_start]
        target_text = text[col_start:col_end]
        
        if 'expected `i32`, found `i64`' in expected or 'expected `i32`, found `usize`' in expected or 'expected `i32`, found `u64`' in expected:
            if target_text.endswith('.try_into().unwrap()'): continue
            new_text = f"({target_text}).try_into().unwrap()"
            text = text[:col_start] + new_text + text[col_end:]
            files_to_fix[file_name][line_start] = text
        elif 'expected `i64`, found `i32`' in expected or 'expected `u64`, found `i32`' in expected or 'expected `usize`, found `i32`' in expected or 'expected `usize`, found `u32`' in expected or 'expected `u64`, found `usize`' in expected or 'expected `u64`, found `u32`' in expected:
            # Output types or other way around
            if 'as u64' in target_text or 'as i64' in target_text or 'as usize' in target_text: continue
            
            # Find the expected return type
            if 'expected `u64`' in expected:
                cast = 'as u64'
            elif 'expected `i64`' in expected:
                cast = 'as i64'
            elif 'expected `usize`' in expected:
                cast = 'as usize'
            else:
                cast = 'as _'
                
            new_text = f"({target_text}) {cast}"
            text = text[:col_start] + new_text + text[col_end:]
            files_to_fix[file_name][line_start] = text
            
    for file_name, lines in files_to_fix.items():
        with open(file_name, 'w', encoding='utf-8') as src:
            src.writelines(lines)
        print(f"Fixed {file_name}")

if __name__ == '__main__':
    # run iteratively 3 times to fix nested issues
    for _ in range(3):
        apply_fixes()
