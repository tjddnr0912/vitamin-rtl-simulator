module top;
  logic [1:0] v;
  covergroup cg_t;
    coverpoint v;
  endgroup
  initial begin
    cg_t cg = new();             // block-local declaration of a covergroup-typed variable
    v = 2'd1; cg.sample();
    $display("ok");
    $finish;
  end
endmodule
