module top;
  localparam logic [64:0] P65 = 65'hC;
  function automatic int finc(input int a); return a + 5; endfunction
  localparam int R = finc(P65 ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
