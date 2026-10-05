module top;
  logic a;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    f = v;
  endfunction
  function automatic logic gr(input logic v);
    integer r; r = $random;
    $display("gr t=%0t v=%b", $time, v);
    gr = v;
  endfunction
  wire [1:0] s; wire c1;
  assign s[1] = gr(s[0]);
  assign c1 = f(a, 1);
  assign s[0] = f(c1, 2);
  wire [1:0] q = s;
  initial begin a = 1; #1 $display("t1 s=%b q=%b", s, q); $finish; end
  initial #10 $finish;
endmodule
