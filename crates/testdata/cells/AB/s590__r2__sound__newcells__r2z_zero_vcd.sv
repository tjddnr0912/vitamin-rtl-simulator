module top;
  logic a;
  wire [3:0] w;
  wire v;
  function automatic logic [3:0] p(input logic x); return {4{x}}; endfunction
  assign w = p(a);
  assign v = ^w;
  initial begin $dumpfile("z.vcd"); $dumpvars(0, top); a = 1'b0; #5 a = 1'b1; #5 $finish; end
endmodule
