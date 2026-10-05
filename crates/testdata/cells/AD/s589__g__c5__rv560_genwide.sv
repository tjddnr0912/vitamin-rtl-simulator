module top;
  localparam logic [79:8] K = 72'h036263646566676869;
  function automatic int g(input int a); return K[79:72]; endfunction
  if (1) begin : b
    localparam logic [79:8] K = 72'h077273747576777879;
    localparam int P = g(0);
    initial #1 $display("P=%0d", P);
  end
  initial #2 $finish;
endmodule
