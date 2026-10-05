class C;
  function int f(int x);
    unique case (x) 0: return 1; default: return 2; endcase
  endfunction
endclass
