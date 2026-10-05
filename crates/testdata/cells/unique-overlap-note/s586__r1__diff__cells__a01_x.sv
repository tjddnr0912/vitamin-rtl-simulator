package px;
  function automatic int f(logic [1:0] r);
    unique0 casez (r) 2'b?1: return 1; 2'b1?: return 2; endcase
    return 0;
  endfunction
endpackage
