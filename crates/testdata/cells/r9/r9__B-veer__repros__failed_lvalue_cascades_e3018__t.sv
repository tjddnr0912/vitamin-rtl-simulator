module slv (output reg rdy);
  assign rdy = 1'b1;                        // continuous assign to an output `reg`, its sole writer
endmodule
module top;
  wire r;
  logic [1:0][15:2] q;                      // non-zero-LSB element
  slv u (.rdy(r));
  assign q[0][15:2] = 14'h5;                // refused lvalue (nested select) -> leaves an unresolved chunk
  assign q[1] = 14'h6;
  initial #1 begin $display("r=%b q=%h", r, q); $finish; end
endmodule
