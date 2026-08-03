// Keccak-f[1600] permutation core — one round per clock.
// Written locally for vita-vs-iverilog throughput measurement (public algorithm,
// FIPS 202). Lane-array style (`logic [63:0] st [0:24]`) — the shape real Keccak
// RTL uses, and a different stress from picorv32's 32-bit scalars.
module keccak_f (
    input  wire        clk,
    input  wire        rst_n,
    input  wire        start,
    input  wire [1599:0] din,
    output reg  [1599:0] dout,
    output reg         done
);
    reg [63:0] st [0:24];
    reg [4:0]  rnd;
    reg        busy;

    function automatic [63:0] rotl64 (input [63:0] v, input integer n);
        rotl64 = (n == 0) ? v : ((v << n) | (v >> (64 - n)));
    endfunction

    function automatic [63:0] rc (input [4:0] i);
        case (i)
            5'd0:  rc = 64'h0000000000000001; 5'd1:  rc = 64'h0000000000008082;
            5'd2:  rc = 64'h800000000000808a; 5'd3:  rc = 64'h8000000080008000;
            5'd4:  rc = 64'h000000000000808b; 5'd5:  rc = 64'h0000000080000001;
            5'd6:  rc = 64'h8000000080008081; 5'd7:  rc = 64'h8000000000008009;
            5'd8:  rc = 64'h000000000000008a; 5'd9:  rc = 64'h0000000000000088;
            5'd10: rc = 64'h0000000080008009; 5'd11: rc = 64'h000000008000000a;
            5'd12: rc = 64'h000000008000808b; 5'd13: rc = 64'h800000000000008b;
            5'd14: rc = 64'h8000000000008089; 5'd15: rc = 64'h8000000000008003;
            5'd16: rc = 64'h8000000000008002; 5'd17: rc = 64'h8000000000000080;
            5'd18: rc = 64'h000000000000800a; 5'd19: rc = 64'h800000008000000a;
            5'd20: rc = 64'h8000000080008081; 5'd21: rc = 64'h8000000000008080;
            5'd22: rc = 64'h0000000080000001; 5'd23: rc = 64'h8000000080008008;
            default: rc = 64'd0;
        endcase
    endfunction

    function automatic integer rho (input integer x, input integer y);
        case (x + 5*y)
            0: rho=0;   1: rho=1;   2: rho=62;  3: rho=28;  4: rho=27;
            5: rho=36;  6: rho=44;  7: rho=6;   8: rho=55;  9: rho=20;
            10: rho=3;  11: rho=10; 12: rho=43; 13: rho=25; 14: rho=39;
            15: rho=41; 16: rho=45; 17: rho=15; 18: rho=21; 19: rho=8;
            20: rho=18; 21: rho=2;  22: rho=61; 23: rho=56; 24: rho=14;
            default: rho=0;
        endcase
    endfunction

    integer x, y;
    reg [63:0] c   [0:4];
    reg [63:0] d   [0:4];
    reg [63:0] a   [0:24];
    reg [63:0] b   [0:24];

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            busy <= 1'b0; done <= 1'b0; rnd <= 5'd0;
            for (x = 0; x < 25; x = x + 1) st[x] <= 64'd0;
        end else begin
            done <= 1'b0;
            if (start && !busy) begin
                for (x = 0; x < 25; x = x + 1) st[x] <= din[64*x +: 64];
                rnd  <= 5'd0;
                busy <= 1'b1;
            end else if (busy) begin
                // theta
                for (x = 0; x < 5; x = x + 1)
                    c[x] = st[x] ^ st[x+5] ^ st[x+10] ^ st[x+15] ^ st[x+20];
                for (x = 0; x < 5; x = x + 1)
                    d[x] = c[(x+4)%5] ^ rotl64(c[(x+1)%5], 1);
                for (y = 0; y < 5; y = y + 1)
                    for (x = 0; x < 5; x = x + 1)
                        a[x + 5*y] = st[x + 5*y] ^ d[x];
                // rho + pi
                for (y = 0; y < 5; y = y + 1)
                    for (x = 0; x < 5; x = x + 1)
                        b[y + 5*((2*x + 3*y) % 5)] = rotl64(a[x + 5*y], rho(x, y));
                // chi
                for (y = 0; y < 5; y = y + 1)
                    for (x = 0; x < 5; x = x + 1)
                        a[x + 5*y] = b[x + 5*y] ^ ((~b[((x+1)%5) + 5*y]) & b[((x+2)%5) + 5*y]);
                // iota
                a[0] = a[0] ^ rc(rnd);
                for (x = 0; x < 25; x = x + 1) st[x] <= a[x];
                if (rnd == 5'd23) begin
                    busy <= 1'b0;
                    done <= 1'b1;
                    for (x = 0; x < 25; x = x + 1) dout[64*x +: 64] <= a[x];
                end else begin
                    rnd <= rnd + 5'd1;
                end
            end
        end
    end
endmodule
