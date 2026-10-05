module child #(parameter type T = logic [3:0]) (output int n);
  function automatic logic [3:0] fd(input int a);
    T t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  initial begin n = 0; repeat (fd(2)) n = n + 1; end
endmodule
module top;
  int n;
  child #(.T(bit [3:0])) u (.n(n));
  initial begin #1 $display("n=%0d", n); $display("done"); $finish; end
  initial #100 $finish;
endmodule
