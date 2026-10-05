module top;
  function automatic logic [63:0] f(input int a);
    time t;
    f = t;
  endfunction
  localparam logic [63:0] P = f(2);
  initial begin #1 $display("P=%h", P); $finish; end
  initial #100 $finish;
endmodule
