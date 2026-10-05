package p;
  function automatic int f(int x);
    unique case (x) 0: return 1; 0: return 3; default: return 2; endcase
  endfunction
endpackage
