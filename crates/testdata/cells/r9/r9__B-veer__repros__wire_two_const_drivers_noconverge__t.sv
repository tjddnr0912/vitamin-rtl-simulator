module top;
  wire v;
  assign v = '0;            // two whole-net constant drivers of one wire, same value
  assign v = '0;
  wire y = ~v;
  initial #1 begin $display("v=%b y=%b", v, y); $finish; end
endmodule
