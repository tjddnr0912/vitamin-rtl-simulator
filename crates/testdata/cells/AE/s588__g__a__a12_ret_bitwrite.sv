module top;
  function automatic logic [3:0] f(input int a);
    f[1] = 1'b1;
    if (a == 1) f = 4'd10;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
