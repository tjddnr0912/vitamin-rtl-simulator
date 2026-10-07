module m #() ();                 // empty parameter port list and empty port list (IEEE 1800-2017 A.1.3)
  initial $display("m ok");
endmodule
module top;
  m u ();
  initial #1 $finish;
endmodule
