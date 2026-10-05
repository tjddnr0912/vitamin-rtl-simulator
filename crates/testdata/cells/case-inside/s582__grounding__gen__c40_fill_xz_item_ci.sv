module top;
  logic [35:0] v; int m;
  initial begin
    v = 36'h1; m = 9;
    case (v) inside
      'bx1: m = 1;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 36'h3; m = 9;
    case (v) inside
      'bx1: m = 1;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 36'h2; m = 9;
    case (v) inside
      'bx1: m = 1;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    #10 $finish;
  end
endmodule
