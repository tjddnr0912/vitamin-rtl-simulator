module top;
  wire [1:0] w;
  assign w = '0;           // two continuous assigns to one wire net (IEEE 1800-2017 6.5: a net may have several drivers)
  assign w[1:0] = '0;
  wire [1:0] v;
  assign v = 2'b01;
  assign v[1:0] = 2'b11;   // conflicting values resolve per bit: 2'bx1
  initial #1 begin $display("w=%b v=%b", w, v); $finish; end
endmodule
