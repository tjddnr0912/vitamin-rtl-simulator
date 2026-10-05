module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  typedef enum int {A = f(2), B} e_t;
  logic [15:0] x = 16'h1234, y;
  if (1) begin : g
    $info("el=%0d", f(2));
  end
  initial begin
    #1 y = {<< f(2) {x}};
    $display("cw=%0d st=%h A=%0d B=%0d", f(2)'(9'h1FF), y, A, B);
    #1 $finish;
  end
endmodule
