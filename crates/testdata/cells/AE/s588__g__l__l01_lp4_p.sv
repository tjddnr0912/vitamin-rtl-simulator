module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fx = t;
  endfunction
  localparam logic [3:0] P = fx(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
