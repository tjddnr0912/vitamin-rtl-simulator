module top;
  function automatic logic signed [3:0] f(input int a);
    logic signed [3:0] t;
    f = t;
  endfunction
  localparam logic signed [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
