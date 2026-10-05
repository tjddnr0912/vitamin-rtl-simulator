module top;
  function automatic int fz(input logic [3:0] a);
    fz = 7;
    if (a == 4'd1) fz = 10;
    else if (a == 4'd2) fz = 11;
  endfunction
  localparam int P = fz(4'bx);
  localparam int Q = fz(4'bz01x);
  initial begin #1 $display("P=%0d Q=%0d", P, Q); $finish; end
endmodule
