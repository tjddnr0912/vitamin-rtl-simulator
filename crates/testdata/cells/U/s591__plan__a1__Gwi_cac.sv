package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    sac #(.N(i)) u7 ();
  end
  initial #100 $finish;
endmodule
module sac #(parameter N = 0) ();
  function automatic integer fc(input integer x); case (x) 0: fc = 3; 1: fc = 5; default: fc = 7; endcase endfunction
  wire [7:0] r = {fc(N){1'b1}};
  initial #3 $display("ac %m r=%b", r);
endmodule
