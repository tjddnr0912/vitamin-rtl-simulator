module top;
  logic [7:0] v8; logic [3:0] a4, b4; int m;
  initial begin
    v8 = 8'h10; a4 = 4'h8; b4 = 4'h8;
    m = (v8 == (a4 + b4)); $display("eq=%0d", m);
    m = (v8 === (a4 + b4)); $display("ceq=%0d", m);
    m = (v8 inside {a4 + b4}); $display("ins=%0d", m);
    case (v8) a4 + b4: m = 1; default: m = 0; endcase $display("case=%0d", m);
    m = (v8 >= (a4 + b4)) && (v8 <= (a4 + b4)); $display("rng=%0d", m);
    #10 $finish;
  end
endmodule
