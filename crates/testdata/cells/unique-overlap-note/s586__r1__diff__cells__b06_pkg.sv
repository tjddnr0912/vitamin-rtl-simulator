package pk;
  function automatic int pick(logic [1:0] r);
    unique0 casez (r)
      2'b?1: return 1;
      2'b1?: return 2;
    endcase
    return 0;
  endfunction
endpackage
module top;
  import pk::*;
  initial #100 $finish;
  initial #1 $display("pick=%0d", pick(2'b11));
endmodule
