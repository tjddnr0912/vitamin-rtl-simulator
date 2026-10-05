module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t = 4'd3;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
