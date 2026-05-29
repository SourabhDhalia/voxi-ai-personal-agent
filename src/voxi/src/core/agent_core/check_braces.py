import sys

def check_delimiters(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    stack = []
    pairs = {')': '(', '}': '{', ']': '['}
    
    in_string = False
    in_comment = False
    escape = False
    
    line_no = 1
    col_no = 1
    
    i = 0
    while i < len(content):
        char = content[i]
        
        if char == '\n':
            line_no += 1
            col_no = 1
            in_comment = False
            i += 1
            continue
        
        if in_comment:
            col_no += 1
            i += 1
            continue
            
        if in_string:
            if escape:
                escape = False
            elif char == '\\':
                escape = True
            elif char == '"':
                in_string = False
            col_no += 1
            i += 1
            continue
            
        # Check for comments
        if char == '/' and i + 1 < len(content) and content[i+1] == '/':
            in_comment = True
            col_no += 2
            i += 2
            continue
            
        # Check for strings
        if char == '"':
            in_string = True
            col_no += 1
            i += 1
            continue
            
        # Check for char literal
        if char == "'" and i + 2 < len(content) and content[i+2] == "'":
            col_no += 3
            i += 3
            continue
            
        if char in '({[':
            stack.append((char, line_no, col_no, i))
        elif char in ')}]':
            expected = pairs[char]
            if not stack:
                print(f"Error: Unmatched closing '{char}' at line {line_no}, col {col_no}")
                return
            
            top_char, top_line, top_col, top_idx = stack[-1]
            if 1677 <= line_no <= 2590 or 1677 <= top_line <= 2590:
                print(f"Match: '{top_char}' at line {top_line}, col {top_col} matched with '{char}' at line {line_no}, col {col_no}")
                
            if top_char != expected:
                print(f"Mismatched delimiter: opened '{top_char}' at line {top_line}, col {top_col} but closed with '{char}' at line {line_no}, col {col_no}")
                return
            stack.pop()
        
        col_no += 1
        i += 1

if __name__ == '__main__':
    check_delimiters('/Users/sdhalia/Developer/githubRepo/shoppingAgent/src/voxi/src/core/agent_core/process_prompt.rs')
