module top;
  function automatic bit [3:0] f(input int a);
    if (a == 1) f = 4'd3;
  endfunction
  localparam bit [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
