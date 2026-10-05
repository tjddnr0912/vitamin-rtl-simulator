module top;
  int m; logic [3:0] v;
  function automatic logic [3:0] g(input int k); $display("g(%0d)", k); return k[3:0]; endfunction
  initial begin
    v = 4'd5;
    case (v) g(1), g(2): m = 1; g(5): m = 3; g(6): m = 4; default: m = 0; endcase
    $display("m=%0d", m);
    #10 $finish;
  end
endmodule
