module top;
  import "DPI-C" function int abs(input int x);   // libc symbol, no C file needed
  initial begin
    $display("abs=%0d", abs(-5));
    $finish;
  end
endmodule
