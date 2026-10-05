module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  child #(.P(fd(2))) u();
  initial begin #2 $display("done"); $finish; end
  initial #100 $finish;
endmodule
module child #(parameter logic [3:0] P = 4'd9) ();
  initial #1 $display("P=%0d", P);
endmodule
