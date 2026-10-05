package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::i;
  for (genvar i = 0; i < 2; i++) begin : g
    sacr #(.N(i)) u8 (.a('0));
  end
  initial #100 $finish;
endmodule
module sacr #(parameter N = 0) (input logic [N:0] a);
  function automatic integer fc(input integer x); case (x) 0: fc = 3; 1: fc = 5; default: fc = 7; endcase endfunction
  wire [7:0] r = {fc(N){1'b1}};
  initial #3 $display("acr %m r=%b b=%0d", r, $bits(a));
endmodule
