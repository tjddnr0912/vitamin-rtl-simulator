package pk;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
endpackage
module top;
  import pk::*;
  localparam logic [3:0] P = fd(2);
  initial begin #2 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
