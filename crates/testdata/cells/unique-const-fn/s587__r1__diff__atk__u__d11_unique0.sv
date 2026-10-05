module top;
  function automatic int f0(input int a);
    f0 = 7;
    unique0 if (a == 1) f0 = 10;
  endfunction
  localparam int P = f0(2);
  int n = 0;
  initial begin repeat (f0(2)) n++; #1 $display("P=%0d n=%0d", P, n); $finish; end
endmodule
