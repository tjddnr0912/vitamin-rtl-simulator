module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a)
      1: f = 10;
      3: f = 30;
    endcase
  endfunction
  localparam int P = f(2);
  localparam int Q = f(1);
  initial begin #1 $display("P=%0d Q=%0d", P, Q); $finish; end
endmodule
