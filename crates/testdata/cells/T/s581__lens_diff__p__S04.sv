module top;
  let f(a) = (a ==? 'bx1);
  let g(a) = (a inside {'b?0});
  localparam L1 = f(8'hFF);
  localparam L2 = f(64'hFFFF_FFFF_FFFF_FFFE);
  localparam L3 = g(8'h02);
  localparam L4 = f(65'h1_0000_0000_0000_0001);
  initial $display("@L %b%b%b%b", L1, L2, L3, L4);
  initial $display("@R %b%b%b%b", f(8'hFF), f(64'hFFFF_FFFF_FFFF_FFFE), g(8'h02), f(65'h1_0000_0000_0000_0001));
endmodule
