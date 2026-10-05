module top;
  logic a = 1'b0, b = 1'b1; logic [1:0] y, z;
  function logic [1:0] f(input logic x, input logic w);
    f = 0;
    unique case ({x, w}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  assign y = f(a, b);
  always_comb z = y;
  final $display("final y=%b z=%b", y, z);
endmodule
